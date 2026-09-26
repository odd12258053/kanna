//! `cargo +nightly fuzz run lexer` (from `kanna-core/`).
//!
//! Feeds arbitrary command lines and accessor scripts through the shared
//! invariant harness in `tests/common/harness.rs`.
#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../tests/common/harness.rs"]
#[allow(dead_code)]
mod harness;

#[derive(arbitrary::Arbitrary, Debug)]
struct Input {
    args: Vec<Vec<u8>>,
    script: Vec<u8>,
}

fuzz_target!(|input: Input| {
    harness::exercise(&input.args, &input.script);
});
