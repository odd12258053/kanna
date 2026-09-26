# ADR-0001: Workspace layout and crate boundaries

Status: Accepted (2026-09-26)

## Context

SPEC.md asks for a layered library: a dependency-free lexer at the bottom,
optional declarative/help/derive layers on top, and separately compiled
generators for completions, documentation and JSON schema. The build-time
budget requires that `syn` never enters the dependency graph unless the user
opts in.

## Decision

One Cargo workspace (resolver 3, edition 2024, MSRV 1.85) with these members:

| Crate | Role | Dependencies |
|-------|------|--------------|
| `kanna-core` | Lexer. One source file, `#![forbid(unsafe_code)]` (enforced workspace-wide). | none |
| `kanna` | Facade: declarative `Command` IR, help, errors, constraints, `cli!` macro, feature-gated re-exports. | `kanna-core` |
| `kanna-derive` | `#[derive(Args)]` proc-macro. | `syn`, `quote`, `proc-macro2` |
| `kanna-complete` | Shell completion generation. | `kanna` |
| `kanna-doc` | manpage / Markdown / HTML generation. | `kanna` |
| `kanna-schema` | JSON schema output. | `kanna` |
| `benches` (package `kanna-benches`) | Size / build-time samples and comparison benches. Never published. | whatever it compares against |

Feature flags live on the `kanna` facade; each extra crate is pulled in only
by its feature (`derive`, `complete`, `doc`, `schema`). The default feature set
is `["help", "std"]`.

Workspace-wide lints: `unsafe_code = "forbid"`, `missing_docs = "warn"`,
`clippy::all = "warn"`. CI runs clippy with `-D warnings`.

A `size` Cargo profile (`opt-level = "s"`, `lto = true`, `codegen-units = 1`,
`panic = "abort"`, `strip = true`) is defined at the workspace root so that
every size measurement uses identical settings (see ADR-0007).

## Consequences

* `cargo build -p kanna-core` compiles exactly one crate. Anyone can vendor
  `kanna-core/src/lib.rs` as a single file.
* The `benches` package may depend on `clap`, `lexopt`, etc. Because it is a
  separate package and resolver 3 does not unify features across packages
  that are not built together, this never affects what `kanna` users
  compile.
* `kanna-derive` must stay behind the `derive` feature; nothing in the
  default graph may depend on it.
