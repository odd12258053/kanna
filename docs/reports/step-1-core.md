# Step 1 report: `hasami-core`

Date: 2026-09-26. Toolchain: rustc 1.98.1 stable (x86_64-unknown-linux-gnu),
MSRV check with 1.85, fuzzing with nightly.

## What was built

* `hasami-core/src/lib.rs`: the lexer, 857 lines including docs and unit
  tests. Zero dependencies, `#![forbid(unsafe_code)]`.
* `hasami-core/tests/lexer.rs`: 66 behavioural tests covering every row of
  ADR-0003 (attached/separate/optional/multiple values, clusters, `--`, `-`,
  empty strings, `--opt=`, `-o=val`, error recovery, non-UTF-8 bytes on Unix).
* `hasami-core/tests/pseudo_fuzz.rs`: 200 000 PRNG-generated command lines
  through the invariant harness on stable, in 0.5 s.
* `hasami-core/fuzz/`: libFuzzer target sharing the same harness.

## Measurements

| Check | Result |
|-------|--------|
| `cargo test -p hasami-core` | 70 tests + 1 doctest pass |
| `cargo +1.85 test -p hasami-core --no-default-features` | pass |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo doc` with `-D warnings` | clean |
| `cargo +nightly fuzz run lexer -- -max_total_time=90` | 3 914 552 runs, 0 crashes, 437 edges covered |
| Size delta of sample CLI vs empty baseline (`size` profile) | **13.5 KiB** (budget: 20 KiB) |
| Clean build of `hasami-core` (dev / release) | 0.17 s / 0.23 s wall (budget: 2 s) |
| Clean build of sample CLI, `size` profile (LTO) | 2.4 s wall |

## Design points that needed a decision

* **`-o=val` strips the `=`** (ADR-0003). GNU `getopt` would give `=val`;
  lexopt strips. Users overwhelmingly mean the separator, and `-o "=val"`
  as a separate argument still works for the rare literal case.
* **`values()` stops after one value when `=` was used** (ADR-0003), so a
  user can always write `--exec=cmd` to prevent gobbling.
* **Non-UTF-8 short flags become `U+FFFD`** rather than an error
  (ADR-0004). Making the lexer total (never erroring on shape) keeps the
  state machine simple and lets higher layers decide what to say.
* **No `unsafe` for Windows**: a 30-line WTF-8 to UTF-16 routine avoids
  `from_encoded_bytes_unchecked` (ADR-0004). It is unit-tested on every
  platform but not yet exercised end-to-end on Windows.
* **Arguments are collected into a `Vec`** rather than boxing an iterator
  (ADR-0005) to get lookahead cheaply. Command lines are small.
* **`Error` is `#[non_exhaustive]`** and carries boxed sources so the
  declarative layer can wrap it without redesign.
* **`std` is a feature that cannot currently be turned off** (ADR-0002):
  `OsString` needs `std`; the flag reserves the door for an `alloc` mode.

## Open items carried forward

* Windows CI job to run the non-Unicode tests with real `OsString`s.
* CI wiring for the size gate and the timed fuzz run (step 4).
