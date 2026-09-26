//! Environment variable fallback (feature `env`).
//!
//! Mutating the process environment is `unsafe` on edition 2024 and the
//! workspace forbids `unsafe`, so the assertions run in a child process
//! started with a controlled environment.
#![cfg(feature = "env")]

use std::process::Command as Process;

use kanna::{Arg, Command, ErrorKind, Source};

fn cmd() -> (Command, Arg<u16>) {
    let port = Arg::new("port")
        .value::<u16>()
        .env("KANNA_TEST_PORT")
        .default(80);
    let cmd = Command::new("x").arg(&port);
    (cmd, port)
}

/// Entry point in the child: `CHILD=<case>` selects the assertions.
fn child(case: &str) {
    let (cmd, port) = cmd();
    match case {
        "present" => {
            let m = cmd.try_parse_args([] as [&str; 0]).unwrap();
            assert_eq!(m.get(&port), 8080);
            assert_eq!(m.source(&port), Some(Source::Env));
            assert!(m.contains(&port));
            let m = cmd.try_parse_args(["--port", "1"]).unwrap();
            assert_eq!(m.get(&port), 1);
            assert_eq!(m.source(&port), Some(Source::CommandLine));
        }
        "invalid" => {
            let e = cmd.try_parse_args([] as [&str; 0]).unwrap_err();
            assert_eq!(e.kind(), ErrorKind::InvalidValue);
            assert!(e.message().contains("'zzz'"));
        }
        "absent" => {
            let m = cmd.try_parse_args([] as [&str; 0]).unwrap();
            assert_eq!(m.get(&port), 80);
            assert_eq!(m.source(&port), Some(Source::Default));
            assert!(!m.contains(&port));
        }
        other => panic!("unknown case {other}"),
    }
}

fn run_child(case: &str, var: Option<&str>) {
    let mut p = Process::new(std::env::current_exe().expect("test binary path"));
    p.arg("--exact").arg("child_entry").arg("--nocapture");
    p.env("KANNA_ENV_CASE", case);
    p.env_remove("KANNA_TEST_PORT");
    if let Some(v) = var {
        p.env("KANNA_TEST_PORT", v);
    }
    let out = p.output().expect("spawn child");
    assert!(
        out.status.success(),
        "child {case} failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn child_entry() {
    if let Ok(case) = std::env::var("KANNA_ENV_CASE") {
        child(&case);
    }
}

#[test]
fn env_value_is_used_when_argument_absent() {
    run_child("present", Some("8080"));
}

#[test]
fn env_value_is_validated() {
    run_child("invalid", Some("zzz"));
}

#[test]
fn default_applies_when_env_absent() {
    run_child("absent", None);
}

#[cfg(feature = "help")]
#[test]
fn env_is_shown_in_help() {
    let port = Arg::new("port").value::<u16>().env("APP_PORT").help("Port");
    let cmd = Command::new("x").arg(&port);
    assert!(cmd.render_help().contains("Port [env: APP_PORT]"));
    assert_eq!(cmd.find_arg("port").unwrap().env(), Some("APP_PORT"));
}
