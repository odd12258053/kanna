# Step 8 report: documentation, examples, final verification

Date: 2026-09-26. Machine and toolchain as in the step 4/5 report.

## What was built

* `README.md`: layer table, "which layer" decision flow, one example per
  layer, feature table, comparison table, principles, links.
* `CHANGELOG.md` (Keep a Changelog), `LICENSE-MIT`, `LICENSE-APACHE`.
* `docs/migration/`: guides from clap, argh, bpaf, lexopt, pico-args, xflags.
* `hasami/examples/`: `core`, `builder`, `macro`, `derive`, `extras`.
* `hasami/tests/snapshots.rs` + `tests/cmd/*.toml`: trycmd snapshots of
  help, version, success and five error shapes on the `builder` example.
* ADR-0016 (generators); ADR index complete through 0016.

## Final verification (all green)

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo test --workspace` default / `--all-features` / `--no-default-features` / `--features derive` | all pass (core 66 + 2 pseudo-fuzz + 3 unit; builder 45; macro 9; derive 4; env 5; color 7; suggest 5; no-help 2; snapshots 8 cases; complete 9; doc 4; schema 4; doctests) |
| `cargo clippy --workspace --all-targets` with `--all-features` and `--no-default-features`, `-D warnings` | clean |
| `cargo doc --workspace --no-deps --all-features` with `-D warnings` | clean |
| `cargo +1.85 check --workspace --all-features --all-targets` (MSRV) | pass |
| `cargo deny check` | advisories, bans, licenses, sources ok |
| Third-party dependencies of `hasami` with default features | 0 |
| `benches/size.sh --check --compare` | pass: core 13.5 KiB (≤ 20), builder+help 58.1 KiB (≤ 60), builder full / clap = 0.36 (< 0.5) |
| `benches/build-time.sh --check` | pass: core 0.61 s (≤ 2), builder full 0.73 s (≤ 4) |
| `cargo +nightly fuzz run lexer` 30 s | 1.76 M runs, 0 crashes |

## Design points that needed a decision

* **Snapshots through trycmd on an example binary.** trycmd only finds
  `[[bin]]` targets by itself; the test registers the `builder` example
  with `trycmd::cargo::compile_example`. A deliberate mismatch was used to
  confirm the cases really run and fail.
* **Help output ends with exactly one newline.** `Error::print` no longer
  appends a newline when the rendered text already ends with one; the
  version line still gets one.
* **cargo-deny ignores dev-dependencies** (`exclude-dev = true`): trycmd
  brings syn and serde into the dev graph, which is irrelevant to users.

## Not done / follow-ups

* CI has not been executed on GitHub yet (no remote); the workflow is
  written and every job's command has been run locally on Linux.
* Windows and macOS runs of the non-Unicode tests.
* Multi-value per occurrence (`--exec cmd arg arg`) in the builder.
* Terminal-width wrapping of help text (deliberately out of scope, ADR-0010).
* Nothing has been committed to git; the repository is initialised and
  the tree is ready for a first commit.
