//! Compile-fail tests: an unknown setting in `cli!` or `#[kanna(...)]`
//! must produce an error that names the setting and lists the valid ones,
//! so that a wrong guess is corrected in one step.
//!
//! Update the expected output with `TRYBUILD=overwrite cargo test --features derive --test ui`.

#[test]
fn unknown_settings_are_named() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
