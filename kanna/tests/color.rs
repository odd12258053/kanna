//! Colour output (feature `color`): rendering with a palette, and the
//! environment rules, which run in child processes because the environment
//! cannot be mutated safely in-process.
#![cfg(all(feature = "color", feature = "help"))]

use std::process::Command as Process;

use kanna::{Arg, Command, Stream, Styles};

fn cmd() -> Command {
    let n = Arg::new("number")
        .short('n')
        .value::<u32>()
        .help("How many");
    let t = Arg::positional::<String>("THING").required().help("What");
    Command::new("greet").about("Greet").arg(&n).arg(&t)
}

#[test]
fn display_is_plain_and_render_is_styled() {
    let e = cmd().try_parse_args(["--nope"]).unwrap_err();
    let plain = e.to_string();
    assert!(!plain.contains('\x1b'));
    assert_eq!(e.render(&Styles::PLAIN), plain);
    let styled = e.render(&Styles::COLORED);
    assert!(styled.starts_with("\x1b[1;31merror:\x1b[0m unexpected argument '--nope' found"));
    assert!(
        styled.contains(
            "\x1b[1;4mUsage:\x1b[0m \x1b[1mgreet\x1b[0m [OPTIONS] \x1b[36m<THING>\x1b[0m"
        )
    );
    assert!(styled.contains("\x1b[1m--help\x1b[0m"));
    assert!(styled.contains("\x1b[32mtip:\x1b[0m"));
}

#[test]
fn help_is_styled_too() {
    let e = cmd().try_parse_args(["-h"]).unwrap_err();
    let styled = e.render(&Styles::COLORED);
    assert!(styled.contains("\x1b[1;4mOptions:\x1b[0m"));
    assert!(styled.contains("\x1b[1m-n, --number\x1b[0m\x1b[36m <NUMBER>\x1b[0m  How many"));
    assert!(styled.contains("\x1b[36m<THING>\x1b[0m  What"));
    // Stripping the escapes gives exactly the plain rendering.
    let stripped = strip(&styled);
    assert_eq!(stripped, e.to_string());
}

fn strip(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Child entry: prints whether colour is wanted for stderr.
#[test]
fn child_entry() {
    if std::env::var_os("KANNA_COLOR_CHILD").is_some() {
        let wanted = Styles::for_stream(Stream::Stderr) == Styles::COLORED;
        eprintln!("wanted={wanted}");
    }
}

fn child(vars: &[(&str, Option<&str>)]) -> bool {
    let mut p = Process::new(std::env::current_exe().expect("test binary"));
    p.arg("--exact").arg("child_entry").arg("--nocapture");
    p.env("KANNA_COLOR_CHILD", "1");
    for k in ["NO_COLOR", "CLICOLOR", "CLICOLOR_FORCE", "TERM"] {
        p.env_remove(k);
    }
    for (k, v) in vars {
        match v {
            Some(v) => p.env(k, v),
            None => p.env_remove(k),
        };
    }
    let out = p.output().expect("spawn");
    let err = String::from_utf8_lossy(&out.stderr);
    match err.lines().find_map(|l| l.strip_prefix("wanted=")) {
        Some("true") => true,
        Some("false") => false,
        _ => panic!("no verdict in child output:\n{err}"),
    }
}

#[test]
fn pipe_gets_no_colour_by_default() {
    // The child's stderr is a pipe.
    assert!(!child(&[]));
}

#[test]
fn clicolor_force_enables_colour_on_a_pipe() {
    assert!(child(&[("CLICOLOR_FORCE", Some("1"))]));
    assert!(!child(&[("CLICOLOR_FORCE", Some("0"))]));
}

#[test]
fn no_color_wins_over_force() {
    assert!(!child(&[
        ("CLICOLOR_FORCE", Some("1")),
        ("NO_COLOR", Some(""))
    ]));
}

#[test]
fn dumb_terminal_and_clicolor_zero_disable() {
    assert!(!child(&[("TERM", Some("dumb"))]));
    assert!(!child(&[("CLICOLOR", Some("0"))]));
}
