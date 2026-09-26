//! Snapshot tests of help and error output through `trycmd`: every
//! `tests/cmd/*.toml` runs the `builder` example with fixed arguments and
//! compares stdout/stderr/status against the recorded files.
//!
//! Update snapshots with `TRYCMD=overwrite cargo test --test snapshots`.

#[test]
fn cli_snapshots() {
    let builder = trycmd::cargo::compile_example("builder", [] as [&str; 0]);
    trycmd::TestCases::new()
        .register_bin("builder", builder)
        .case("tests/cmd/*.toml");
}
