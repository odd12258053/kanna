# Steps 2 and 3 report: `kanna` declarative layer and `cli!` macro

Date: 2026-09-26. Toolchain: rustc 1.98.1 stable, MSRV check with 1.85.

## What was built

* `kanna/src/arg.rs`: `Arg<T>` type-state builder and `ArgDef` (ADR-0008).
* `kanna/src/command.rs`: `Command`, `Subcommand` (eager or lazy), `Group`.
* `kanna/src/parse.rs`: the engine on top of `kanna-core` (ADR-0009):
  typed values parsed at parse time, subcommands with global options,
  required/exclusive/requires constraints, env fallback (feature `env`).
* `kanna/src/help.rs`: help/usage rendering (ADR-0010), feature `help`.
* `kanna/src/error.rs`: `Error { kind, message, tip, usage }`.
* `kanna/src/suggest.rs`: Jaro-Winkler "did you mean" (feature `suggest`).
* `kanna/src/macros.rs` + `macro_support.rs`: the `cli!` DSL (ADR-0011).
* Tests: 45 builder tests, 9 macro tests (including a builder/macro
  equivalence test), 5 env tests (run in a child process because setting
  environment variables is `unsafe` on edition 2024), 2 no-help tests.

## Measurements

| Check | Result |
|-------|--------|
| `cargo test --workspace` (default / `--all-features` / `--no-default-features`) | all pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo doc` with `-D warnings` | clean |
| `cargo +1.85 check --workspace --all-features` | pass |
| Size delta, sample CLI, builder, no features | 49.1 KiB |
| Size delta, builder + `help` | **59.4 KiB** (budget: 60 KiB) |
| Size delta, builder + `help` + `suggest` | 65 KiB |
| Size delta, `cli!` macro + `help` | 65.0 KiB |
| Size delta, `cli!` macro + `full` | 71.9 KiB |

Size profile: `opt-level = "s"`, `lto = true`, `codegen-units = 1`,
`panic = "abort"`, `strip = true`, x86_64-unknown-linux-gnu, relative to a
`main` that only reads `args_os` and prints (ADR-0007).

## Design points that needed a decision

* **The `Arg` is the key** (ADR-0008). Alternatives were stringly-typed
  `get::<T>("name")` (clap) or a separate `Key<T>` returned by
  `Command::arg`. Keeping the definition as the key gives typed reads with
  no extra type and lets builder chaining stay intact.
* **Values parsed eagerly** so that every invalid value is reported before
  the program runs, with the option name; `Matches::get` cannot fail on
  user input.
* **Last occurrence wins** for repeated single-value options (ADR-0009);
  `occurrences()` still exposes the count.
* **Optional values only attach** (`--color=x`, never `--color x`), the
  only unambiguous rule.
* **Size trimming.** The first build measured 60.7 KiB with `help`. Two
  changes brought it to 59 KiB: not boxing the core error as a `source`
  (which linked its `Debug` impl for a chain nobody walks), and building
  messages with slice `concat` instead of `format!` at the many cold error
  sites. Marking error constructors `#[cold] #[inline(never)]` made the
  binary *larger* and was reverted.
* **Macro settings evaluated at runtime** (ADR-0011) rather than resolved
  by a token-level slot record; this keeps the `macro_rules!` code small
  enough to read. Splitting `FieldSpec<T>` into a non-generic `Common`
  saved 5.6 KiB on the macro sample.

## Open items carried forward

* `color` feature (step 5).
* Multi-value per occurrence (`--exec cmd arg arg`, core `values()`) is not
  exposed by the builder yet; `many()` covers repeated occurrences only.
* Terminal-width wrapping of help text is deliberately not done (ADR-0010).
