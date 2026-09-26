//! "Did you mean" tips (feature `suggest`).
#![cfg(all(feature = "suggest", feature = "help"))]

use hasami::{Arg, Command, ErrorKind};

fn cmd() -> Command {
    let number = Arg::new("number").short('n').value::<u32>();
    let shout = Arg::new("shout");
    let mode = Arg::new("mode")
        .value::<String>()
        .possible(["fast", "slow", "medium"]);
    let verbose = Arg::new("verbose").global();
    Command::new("app")
        .arg(&number)
        .arg(&shout)
        .arg(&mode)
        .arg(&verbose)
        .subcommand(Command::new("install"))
        .subcommand(Command::new("uninstall"))
}

#[test]
fn unknown_option_suggests_similar_option() {
    let e = cmd().try_parse_args(["--nubmer", "1"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownOption);
    assert_eq!(e.tip(), Some("a similar argument exists: '--number'"));
    let e = cmd().try_parse_args(["--shuot"]).unwrap_err();
    assert_eq!(e.tip(), Some("a similar argument exists: '--shout'"));
    let e = cmd().try_parse_args(["--hlep"]).unwrap_err();
    assert_eq!(e.tip(), Some("a similar argument exists: '--help'"));
}

#[test]
fn unrelated_option_gets_no_suggestion() {
    let e = cmd().try_parse_args(["--zzzzzzzz"]).unwrap_err();
    assert_eq!(e.tip(), None);
}

#[test]
fn unknown_subcommand_suggests_similar_subcommand() {
    let e = cmd().try_parse_args(["instal"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownSubcommand);
    assert_eq!(e.tip(), Some("a similar subcommand exists: 'install'"));
    let e = cmd().try_parse_args(["zzz"]).unwrap_err();
    assert_eq!(
        e.tip(),
        Some("see 'app --help' for the list of subcommands")
    );
}

#[test]
fn invalid_possible_value_suggests_similar_value() {
    let e = cmd().try_parse_args(["--mode", "fsat"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::InvalidValue);
    assert_eq!(e.tip(), Some("a similar value exists: 'fast'"));
}

#[test]
fn global_options_are_suggested_inside_subcommands() {
    let e = cmd().try_parse_args(["install", "--verbos"]).unwrap_err();
    assert_eq!(e.tip(), Some("a similar argument exists: '--verbose'"));
    // Non-global parent options are not.
    let e = cmd().try_parse_args(["install", "--shuot"]).unwrap_err();
    assert_eq!(e.tip(), None);
}
