# Step 6 report: `hasami-derive`

Date: 2026-09-26. Machine and toolchain as in the step 4/5 report.

## What was built

* `hasami-derive`: `#[derive(Args)]` and `#[derive(Commands)]` with
  `#[hasami(...)]` attributes (ADR-0014); syn 2 without `full`.
* `hasami::Cli` and `hasami::Subcommands` traits shared by the derive and
  the `cli!` macro; `hasami::prelude`.
* `hasami/tests/derive.rs`: equivalence test against `cli!` over 15
  inputs (parses, errors, help for root and subcommands, version), every
  field kind, required subcommands.
* `benches/src/bin/derive.rs`: the sample CLI through the derive.

## Measurements

| Check | Result |
|-------|--------|
| `cargo test --workspace --features derive` / `--all-features` | pass |
| clippy `-D warnings`, `cargo doc -D warnings`, `cargo deny`, MSRV 1.85 | pass |
| Size delta, derive + `help` | 63.4 KiB (identical to the `cli!` sample) |
| Size delta, derive + `full` | 77.8 KiB |
| Clean dev build, derive + `help` | 2.3 s (macro/builder: 0.7 s) |
| Clean release build, derive + `help` | 2.5 s |
| Third-party crates pulled in by `derive` | proc-macro2, quote, syn, unicode-ident |

## Design points that needed a decision

* **Traits instead of inherent methods.** The first `cli!` version
  generated inherent `parse()`/`command()` methods. Moving them to `Cli`
  means one definition of the parsing entry points, one place to document
  them, and generic code can be written over any definition. The price is
  a `use hasami::Cli;` at call sites, as with clap's `Parser`.
* **Raw-token setting values** keep syn small (no `full`): the derive
  never needs to understand the expression, only to paste it into a
  method call.
* **Doc comments are emitted once.** An early version emitted the struct
  doc both as settings and as explicit calls, duplicating the `about`
  line; the equivalence test caught it immediately.

## Open items carried forward

* None specific to the derive; the macro/derive limitations (literal
  `Option<T>`/`Vec<T>` spelling, `usize` counters) are documented on
  `cli!`.
