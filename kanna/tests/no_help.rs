//! Behaviour with the `help` feature disabled: no `--help`/`--version`, no
//! usage lines in errors, everything else identical.
#![cfg(not(feature = "help"))]

use kanna::{Arg, Command, ErrorKind};

#[test]
fn help_and_version_are_plain_unknown_options() {
    let n = Arg::new("number").value::<u32>().required();
    let cmd = Command::new("x").version("1").arg(&n);
    assert_eq!(
        cmd.try_parse_args(["--help"]).unwrap_err().kind(),
        ErrorKind::UnknownOption
    );
    assert_eq!(
        cmd.try_parse_args(["-V"]).unwrap_err().kind(),
        ErrorKind::UnknownOption
    );
    assert!(!cmd.has_help_flag());
    assert!(!cmd.has_version_flag());
}

#[test]
fn errors_have_no_usage() {
    let n = Arg::new("number").value::<u32>().required();
    let cmd = Command::new("x").arg(&n);
    let e = cmd.try_parse_args([] as [&str; 0]).unwrap_err();
    assert_eq!(
        e.to_string(),
        "error: the following required arguments were not provided:\n  --number <NUMBER>\n\nUsage: x"
    );
    let m = cmd.try_parse_args(["--number=4"]).unwrap();
    assert_eq!(m.get(&n), 4);
}
