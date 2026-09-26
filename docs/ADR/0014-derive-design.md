# ADR-0014: `#[derive(Args)]` design and the `Cli` / `Subcommands` traits

Status: Accepted (2026-09-26)

## Context

SPEC.md §4.4 requires the derive to have exactly the semantics of the
`cli!` macro, §2.5 requires `syn` to be isolated so it compiles only when
the `derive` feature is on, and the two front ends must lower to the same
`Command` IR.

## Decision

* **Shared traits.** `hasami::Cli` (structs: `ABOUT`, `command()`,
  `from_matches()`, plus provided `parse`/`try_parse`/`parse_from`/
  `try_parse_from`/`try_parse_args`) and `hasami::Subcommands` (enums:
  `subcommands()`, `from_matches()`). Both `cli!` and the derives implement
  these traits; nothing else is generated except a hidden `__spec()`.
  The traits are named to avoid clashing with the common struct name
  `Args` and with the derive macro of the same name.
* **Same generated code.** The derive emits the same calls into
  `hasami::__macro::{FieldSpec, CommandSpec, SubSpec}` as the `cli!`
  expansion, with the same field classification (`bool`, `usize`,
  `Option<T>`, `Vec<T>`, other; `#[hasami(subcommand)]`). Equivalence is
  therefore by construction, and additionally checked by
  `hasami/tests/derive.rs`, which compares help, usage, matches and error
  text for 15 inputs between a derived and a `cli!` definition. The two
  size-gate samples compile to byte-identical binaries (63.4 KiB).
* **Attribute syntax.** Settings live in `#[hasami(...)]` with the same
  vocabulary as `cli!` (`short`, `short = 'n'`, `default = expr`, ...).
  Setting values are captured as raw token streams up to the next
  top-level comma, so any expression (including `["a", "b"]` arrays and
  `String::from("x")`) is accepted **without** syn's `full` feature. The
  derive compiles syn with only `derive`, `parsing`, `printing`,
  `proc-macro`, `clone-impls`.
* **Isolation.** `hasami-derive` is an optional dependency behind the
  `derive` feature; `deny.toml` bans `syn` anywhere else, and CI checks
  that the default feature set has no third-party dependencies at all.
* Generics on the struct and tuple/unit structs are rejected with a
  spanned compile error; enum variants must be unit-like or carry exactly
  one `Cli` struct.

## Consequences

* Users write `use hasami::{Args, Cli};` (the derive and the trait), or
  `use hasami::prelude::*`.
* Clean build cost of the derive on the reference machine: 2.3 s dev
  (syn + quote + proc-macro2 + unicode-ident) versus 0.7 s without.
