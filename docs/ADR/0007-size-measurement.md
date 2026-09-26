# ADR-0007: How binary size is measured for the size gate

Status: Accepted (2026-09-26)

## Context

SPEC.md §5 sets size budgets (core ≤ 20 KiB, `help` ≤ 60 KiB, all features
≤ half of clap) measured as the difference between a sample CLI and an empty
`main`, built with `opt-level = "s"`, `lto = true`, `panic = "abort"`,
`strip = true`.

## Decision

* The workspace defines a `size` profile with exactly those settings plus
  `codegen-units = 1`, so every measurement uses one configuration.
* `benches/src/bin/empty.rs` is the baseline. It calls
  `std::env::args_os().count()` and `println!`, so the parts of `std` that
  every CLI pays for anyway (argument access, stdout, formatting) are
  excluded from the delta.
* Every other `benches/src/bin/*.rs` implements the **same** sample CLI:
  five options (`-v/--verbose` flag, `-n/--name <STRING>`,
  `-c/--count <U32>`, `-o/--output <PATH>`, `--color[=WHEN]`) and one
  positional `INPUT`. One binary per hasami layer and per competing
  library.
* `benches/size.sh` builds them and prints bytes and delta in KiB. CI runs
  it and fails when a delta exceeds its budget.

## Consequences

* Numbers are only comparable within the same toolchain and target;
  reports record both.
* The baseline is not a truly empty `main`; it is the honest floor for a
  program that reads arguments at all, which is what the budget is for.
