# Architecture Decision Records

Every non-obvious design decision in hasami is recorded here as a short,
numbered ADR. The format follows Michael Nygard's template: **Context**,
**Decision**, **Consequences**, plus a **Status** line.

Numbers are never reused. A superseded ADR keeps its file and gets
`Status: Superseded by ADR-NNNN`.

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-workspace-layout.md) | Workspace layout and crate boundaries | Accepted |
| [0002](0002-core-requires-std.md) | The core requires `std`; `alloc`-only is deferred | Accepted |
| [0003](0003-lexing-rules.md) | Lexing rules for GNU/POSIX edge cases | Accepted |
| [0004](0004-non-unicode-handling.md) | Non-Unicode arguments without `unsafe` | Accepted |
| [0005](0005-core-api-shape.md) | Core API shape: imperative, borrowing `Arg`, sealed `ValueExt` | Accepted |
| [0006](0006-fuzzing-strategy.md) | Fuzzing on nightly plus a stable pseudo-fuzz test | Accepted |
| [0007](0007-size-measurement.md) | How binary size is measured for the size gate | Accepted |
| [0008](0008-decl-ir-and-typed-keys.md) | The declarative IR and typed keys | Accepted |
| [0009](0009-parse-semantics.md) | Parse semantics of the declarative layer | Accepted |
| [0010](0010-help-format.md) | Help output format | Accepted |
| [0011](0011-cli-macro-design.md) | `cli!` macro design (macro_rules, runtime specs) | Accepted |
| [0012](0012-quality-gates.md) | Quality gates and how they are measured | Accepted |
| [0013](0013-color-rules.md) | Colour output rules | Accepted |
| [0014](0014-derive-design.md) | `#[derive(Args)]` design and the `Cli` / `Subcommands` traits | Accepted |
| [0015](0015-generator-crates-depend-on-hasami.md) | Generator crates depend on `hasami`; no re-export features | Accepted |
| [0016](0016-generators.md) | Completion, documentation and schema generators | Accepted |
| [0017](0017-repeats-are-errors-and-definition-validation.md) | Repeated arguments are errors; definitions are validated | Accepted |
| [0018](0018-clap-parity-features.md) | Closing the feature gaps against clap (and the size budget change) | Accepted |
| [0019](0019-ai-oriented-features.md) | Features for CLIs written by, and used by, AI agents | Accepted |
