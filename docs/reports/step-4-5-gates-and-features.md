# Steps 4 and 5 report: quality gates, comparison bench, `suggest` / `color` / `env`

Date: 2026-09-26. Machine: 24-core x86_64 Linux (Fedora, kernel 7.2), rustc
1.98.1 stable. Numbers from `benches/size.sh --compare` and
`benches/build-time.sh --compare` (ADR-0012).

## What was built

* `benches/`: one sample CLI implemented with `hasami-core`, the builder,
  the `cli!` macro, clap 4, lexopt, pico-args, argh and bpaf (combinators);
  `size.sh` and `build-time.sh` with `--check` budgets and `--compare`.
* `.github/workflows/ci.yml`: tests on three OSes, MSRV, feature matrix,
  fmt, clippy, docs, `cargo deny`, zero-dependency check, size gate,
  build-time gate, 60 s fuzz run. `deny.toml` bans `syn` outside
  `hasami-derive`.
* `suggest`: Jaro-Winkler (threshold 0.8) for unknown options, subcommands
  and restricted values; global options are candidates inside subcommands.
* `env`: `Arg::env` / `env = "VAR"`, consulted only when the argument is
  absent, validated like any value, shown as `[env: VAR]` in help.
* `color`: `Styles`, `Stream`, `Error::render`; rules in ADR-0013; seven
  tests including child-process checks of the environment rules.

## Size (delta over the empty baseline, `size` profile)

| Variant | Delta | Budget |
|---------|------:|-------:|
| `hasami-core` | 13.5 KiB | ≤ 20 KiB |
| builder, no features | 48 KiB | |
| builder + `help` | **57.8 KiB** | ≤ 60 KiB |
| builder + `help` + `color` | 65.4 KiB | |
| builder + `full` | 74.8 KiB | < ½ clap |
| `cli!` macro + `help` | 63.4 KiB | |
| `cli!` macro + `full` | 80.6 KiB | |
| pico-args | 11.5 KiB | |
| argh (derive, help) | 15.7 KiB | |
| lexopt | 16.6 KiB | |
| bpaf (combinators) | 95.1 KiB | |
| clap 4 (builder, help + usage + error-context) | 199.0 KiB | |

`builder[full] / clap = 0.38`.

## Clean build time (dev / release, seconds)

| Variant | dev | release | Budget (dev) |
|---------|----:|--------:|-------------:|
| `hasami-core` | 0.61 | 0.84 | ≤ 2 |
| builder + `help` | 0.67 | 1.02 | |
| builder + `full` | 0.73 | 1.08 | ≤ 4 |
| `cli!` macro + `full` | 0.73 | 1.10 | |
| lexopt | 0.62 | 0.89 | |
| pico-args | 0.65 | 0.87 | |
| bpaf | 0.68 | 1.10 | |
| clap | 1.79 | 2.60 | |
| argh (syn) | 3.20 | 3.36 | |

Third-party dependency counts: hasami 0, lexopt 1, pico-args 1, bpaf 1,
clap 4, argh 13.

## Design points that needed a decision

* **Per-library Cargo features for the comparison.** A single `compare`
  feature made lexopt appear to take 3.4 s to build because it dragged in
  syn through argh. Each competitor now has its own feature.
* **Colour that costs nothing when off.** The first colour plumbing added
  2 KiB to the `help` build (60.0 KiB, over budget by rounding). Making
  every `Styles` accessor a constant `""` without the feature, and storing
  help rows as (literal, placeholder, help) triples instead of building a
  second styled string, brought it to 57.8 KiB. Two experiments made it
  worse and were reverted: `#[inline(never)]` on builder methods (+0.4 KiB)
  and a `Box<dyn Write>` in `Error::print` (+7 KiB, vtables pull in
  `write_fmt`/`write_vectored` machinery).
* **The colour context is a cloned `Command`** kept inside the `Error`
  only with the feature on, so help/usage can be re-rendered with escapes
  at print time while `Display` stays plain.
* **Suggestion threshold 0.8** with Jaro-Winkler: catches transpositions
  and one-letter slips (`--nubmer`, `--shuot`, `instal`) without proposing
  unrelated names; `-n` is never suggested for a long name.

## Open items carried forward

* Windows and macOS CI runs are configured but not yet executed here.
* The `help` layer is ~4× lexopt/argh; the runtime IR is the price. The
  largest single function is the parse engine (17.8 KiB); further trimming
  is possible but not required by the budget.
