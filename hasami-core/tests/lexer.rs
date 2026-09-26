//! Behavioural tests for every syntax pattern the lexer must handle.
//!
//! Each test drives the parser the way a real program would, so the tests
//! double as documentation of the intended semantics.

use std::ffi::OsString;

use hasami_core::prelude::*;
use hasami_core::{Error, Parser};

/// Owned, comparable rendering of what the parser produced.
#[derive(Debug, PartialEq, Eq)]
enum Tok {
    S(char),
    L(String),
    V(String),
    /// A value taken with `Parser::value()` after an option.
    Val(String),
    Fail(String),
}

fn os(s: &str) -> OsString {
    OsString::from(s)
}

fn lossy(v: OsString) -> String {
    v.to_string_lossy().into_owned()
}

/// Lex everything, treating every option as a flag (never taking values).
fn flags(args: &[&str]) -> Vec<Tok> {
    let mut p = Parser::from_args(args.iter().copied());
    let mut out = Vec::new();
    loop {
        match p.next() {
            Ok(Some(Short(c))) => out.push(Tok::S(c)),
            Ok(Some(Long(l))) => out.push(Tok::L(l.to_owned())),
            Ok(Some(Value(v))) => out.push(Tok::V(lossy(v))),
            Ok(None) => break,
            Err(e) => out.push(Tok::Fail(e.to_string())),
        }
    }
    out
}

/// Lex everything, treating the options in `takes_value` as options with a
/// required value (short names as single chars, long names without dashes).
fn with_values(args: &[&str], takes_value: &[&str]) -> Vec<Tok> {
    let mut p = Parser::from_args(args.iter().copied());
    let mut out = Vec::new();
    loop {
        let wants = match p.next() {
            Ok(Some(Short(c))) => {
                out.push(Tok::S(c));
                takes_value.iter().any(|t| t.len() == 1 && t.starts_with(c))
            }
            Ok(Some(Long(l))) => {
                out.push(Tok::L(l.to_owned()));
                takes_value.contains(&l)
            }
            Ok(Some(Value(v))) => {
                out.push(Tok::V(lossy(v)));
                false
            }
            Ok(None) => break,
            Err(e) => {
                out.push(Tok::Fail(e.to_string()));
                false
            }
        };
        if wants {
            match p.value() {
                Ok(v) => out.push(Tok::Val(lossy(v))),
                Err(e) => out.push(Tok::Fail(e.to_string())),
            }
        }
    }
    out
}

use Tok::*;

fn l(s: &str) -> Tok {
    L(s.to_owned())
}
fn v(s: &str) -> Tok {
    V(s.to_owned())
}
fn val(s: &str) -> Tok {
    Val(s.to_owned())
}
fn err(s: &str) -> Tok {
    Fail(s.to_owned())
}

// ---------------------------------------------------------------- basics

#[test]
fn empty_command_line() {
    assert_eq!(flags(&[]), vec![]);
}

#[test]
fn next_keeps_returning_none_after_end() {
    let mut p = Parser::from_args(["a"]);
    assert_eq!(p.next().unwrap(), Some(Value(os("a"))));
    assert_eq!(p.next().unwrap(), None);
    assert_eq!(p.next().unwrap(), None);
    assert_eq!(p.next().unwrap(), None);
}

#[test]
fn positional_values() {
    assert_eq!(flags(&["a", "b c", ""]), vec![v("a"), v("b c"), v("")]);
}

#[test]
fn long_flags() {
    assert_eq!(flags(&["--foo", "--bar-baz"]), vec![l("foo"), l("bar-baz")]);
}

#[test]
fn short_flags_and_clusters() {
    assert_eq!(
        flags(&["-a", "-bc", "-d"]),
        vec![S('a'), S('b'), S('c'), S('d')]
    );
}

#[test]
fn short_cluster_with_unicode() {
    assert_eq!(flags(&["-aé日😀"]), vec![S('a'), S('é'), S('日'), S('😀')]);
}

#[test]
fn lone_dash_is_a_value() {
    assert_eq!(flags(&["-", "-a", "-"]), vec![v("-"), S('a'), v("-")]);
}

#[test]
fn double_dash_terminates_options() {
    assert_eq!(
        flags(&["-a", "--", "-b", "--c", "--", "-"]),
        vec![S('a'), v("-b"), v("--c"), v("--"), v("-")]
    );
}

#[test]
fn double_dash_at_end_yields_nothing() {
    assert_eq!(flags(&["-a", "--"]), vec![S('a')]);
}

#[test]
fn triple_dash_is_a_long_option_named_dash() {
    assert_eq!(flags(&["---x"]), vec![l("-x")]);
}

// ---------------------------------------------------------- long values

#[test]
fn long_value_separate() {
    assert_eq!(
        with_values(&["--name", "bob"], &["name"]),
        vec![l("name"), val("bob")]
    );
}

#[test]
fn long_value_attached() {
    assert_eq!(
        with_values(&["--name=bob"], &["name"]),
        vec![l("name"), val("bob")]
    );
}

#[test]
fn long_value_attached_empty() {
    assert_eq!(
        with_values(&["--name="], &["name"]),
        vec![l("name"), val("")]
    );
}

#[test]
fn long_value_attached_with_spaces_and_extra_equals() {
    assert_eq!(
        with_values(&["--name= a=b "], &["name"]),
        vec![l("name"), val(" a=b ")]
    );
}

#[test]
fn long_value_separate_empty_string() {
    assert_eq!(
        with_values(&["--name", ""], &["name"]),
        vec![l("name"), val("")]
    );
}

#[test]
fn long_value_may_look_like_an_option() {
    // GNU getopt behaviour: the next argument is the value, whatever it is.
    assert_eq!(
        with_values(&["--name", "--other", "-x", "--"], &["name"]),
        vec![l("name"), val("--other"), S('x')]
    );
    assert_eq!(
        with_values(&["--name", "--"], &["name"]),
        vec![l("name"), val("--")]
    );
}

#[test]
fn long_value_missing() {
    assert_eq!(
        with_values(&["--name"], &["name"]),
        vec![l("name"), err("missing argument for option '--name'")]
    );
}

#[test]
fn long_flag_given_a_value_is_an_error_and_recovers() {
    assert_eq!(
        flags(&["--verbose=yes", "-x"]),
        vec![
            l("verbose"),
            err("unexpected argument for option '--verbose': \"yes\""),
            S('x')
        ]
    );
}

#[test]
fn long_name_may_be_empty() {
    assert_eq!(with_values(&["--=x"], &[""]), vec![l(""), val("x")]);
    assert_eq!(
        flags(&["--="]),
        vec![l(""), err("unexpected argument for option '--': \"\"")]
    );
}

// --------------------------------------------------------- short values

#[test]
fn short_value_separate() {
    assert_eq!(with_values(&["-n", "5"], &["n"]), vec![S('n'), val("5")]);
}

#[test]
fn short_value_attached() {
    assert_eq!(with_values(&["-n5"], &["n"]), vec![S('n'), val("5")]);
}

#[test]
fn short_value_attached_with_equals_strips_the_equals() {
    assert_eq!(with_values(&["-n=5"], &["n"]), vec![S('n'), val("5")]);
    assert_eq!(with_values(&["-n="], &["n"]), vec![S('n'), val("")]);
    assert_eq!(with_values(&["-n==5"], &["n"]), vec![S('n'), val("=5")]);
}

#[test]
fn short_value_after_cluster() {
    assert_eq!(
        with_values(&["-abn5", "-abn", "6"], &["n"]),
        vec![
            S('a'),
            S('b'),
            S('n'),
            val("5"),
            S('a'),
            S('b'),
            S('n'),
            val("6")
        ]
    );
}

#[test]
fn short_value_is_rest_of_cluster_even_if_it_looks_like_flags() {
    assert_eq!(with_values(&["-nab"], &["n"]), vec![S('n'), val("ab")]);
}

#[test]
fn short_value_missing() {
    assert_eq!(
        with_values(&["-n"], &["n"]),
        vec![S('n'), err("missing argument for option '-n'")]
    );
}

#[test]
fn short_value_may_look_like_an_option() {
    assert_eq!(with_values(&["-n", "-5"], &["n"]), vec![S('n'), val("-5")]);
}

#[test]
fn equals_in_cluster_without_value_is_a_flag_named_equals() {
    // `-a=b` where `a` is a flag: `=` and `b` are simply more short flags.
    assert_eq!(flags(&["-a=b"]), vec![S('a'), S('='), S('b')]);
}

// ------------------------------------------------------- optional values

#[test]
fn optional_value_long() {
    let mut p = Parser::from_args(["--color=always", "--color", "x"]);
    assert_eq!(p.next().unwrap(), Some(Long("color")));
    assert_eq!(p.optional_value().unwrap(), Some(os("always")));
    assert_eq!(p.next().unwrap(), Some(Long("color")));
    assert_eq!(p.optional_value().unwrap(), None);
    assert_eq!(p.next().unwrap(), Some(Value(os("x"))));
}

#[test]
fn optional_value_long_empty() {
    let mut p = Parser::from_args(["--color="]);
    assert_eq!(p.next().unwrap(), Some(Long("color")));
    assert_eq!(p.optional_value().unwrap(), Some(os("")));
    assert_eq!(p.next().unwrap(), None);
}

#[test]
fn optional_value_short() {
    let mut p = Parser::from_args(["-calways", "-c=auto", "-c", "-c"]);
    assert_eq!(p.next().unwrap(), Some(Short('c')));
    assert_eq!(p.optional_value().unwrap(), Some(os("always")));
    assert_eq!(p.next().unwrap(), Some(Short('c')));
    assert_eq!(p.optional_value().unwrap(), Some(os("auto")));
    assert_eq!(p.next().unwrap(), Some(Short('c')));
    assert_eq!(p.optional_value().unwrap(), None);
    assert_eq!(p.next().unwrap(), Some(Short('c')));
    assert_eq!(p.optional_value().unwrap(), None);
    assert_eq!(p.next().unwrap(), None);
}

#[test]
fn optional_value_after_a_positional_is_none() {
    let mut p = Parser::from_args(["x", "y"]);
    assert_eq!(p.next().unwrap(), Some(Value(os("x"))));
    assert_eq!(p.optional_value().unwrap(), None);
    assert_eq!(p.next().unwrap(), Some(Value(os("y"))));
}

#[test]
fn value_after_positional_takes_next_argument() {
    // Not an intended usage, but it must not panic and must be predictable.
    let mut p = Parser::from_args(["x", "y"]);
    assert_eq!(p.next().unwrap(), Some(Value(os("x"))));
    assert_eq!(p.value().unwrap(), os("y"));
    assert!(matches!(
        p.value(),
        Err(Error::MissingValue { option: None })
    ));
}

// ------------------------------------------------------- multiple values

fn collect_values(p: &mut Parser) -> Result<Vec<String>, Error> {
    Ok(p.values()?.map(lossy).collect())
}

#[test]
fn values_gathers_until_next_option() {
    let mut p = Parser::from_args(["--exec", "echo", "hi", "-", "-v", "rest"]);
    assert_eq!(p.next().unwrap(), Some(Long("exec")));
    assert_eq!(collect_values(&mut p).unwrap(), vec!["echo", "hi", "-"]);
    assert_eq!(p.next().unwrap(), Some(Short('v')));
    assert_eq!(p.next().unwrap(), Some(Value(os("rest"))));
}

#[test]
fn values_stops_at_double_dash() {
    let mut p = Parser::from_args(["--exec", "a", "--", "b"]);
    assert_eq!(p.next().unwrap(), Some(Long("exec")));
    assert_eq!(collect_values(&mut p).unwrap(), vec!["a"]);
    assert_eq!(p.next().unwrap(), Some(Value(os("b"))));
    assert_eq!(p.next().unwrap(), None);
}

#[test]
fn values_with_equals_is_limited_to_one() {
    let mut p = Parser::from_args(["--exec=a", "b"]);
    assert_eq!(p.next().unwrap(), Some(Long("exec")));
    assert_eq!(collect_values(&mut p).unwrap(), vec!["a"]);
    assert_eq!(p.next().unwrap(), Some(Value(os("b"))));

    let mut p = Parser::from_args(["-e=a", "b"]);
    assert_eq!(p.next().unwrap(), Some(Short('e')));
    assert_eq!(collect_values(&mut p).unwrap(), vec!["a"]);
    assert_eq!(p.next().unwrap(), Some(Value(os("b"))));
}

#[test]
fn values_attached_short_continues() {
    let mut p = Parser::from_args(["-ea", "b", "-x"]);
    assert_eq!(p.next().unwrap(), Some(Short('e')));
    assert_eq!(collect_values(&mut p).unwrap(), vec!["a", "b"]);
    assert_eq!(p.next().unwrap(), Some(Short('x')));
}

#[test]
fn values_requires_at_least_one() {
    let mut p = Parser::from_args(["--exec", "-x"]);
    assert_eq!(p.next().unwrap(), Some(Long("exec")));
    let e = p.values().expect_err("no values");
    assert_eq!(e.to_string(), "missing argument for option '--exec'");
    // The parser continues normally.
    assert_eq!(p.next().unwrap(), Some(Short('x')));

    let mut p = Parser::from_args(["--exec"]);
    assert_eq!(p.next().unwrap(), Some(Long("exec")));
    assert!(p.values().is_err());
    assert_eq!(p.next().unwrap(), None);
}

#[test]
fn values_may_be_partially_consumed() {
    let mut p = Parser::from_args(["--exec", "a", "b", "c"]);
    assert_eq!(p.next().unwrap(), Some(Long("exec")));
    {
        let mut it = p.values().unwrap();
        assert_eq!(it.next(), Some(os("a")));
    }
    assert_eq!(p.next().unwrap(), Some(Value(os("b"))));
    assert_eq!(p.next().unwrap(), Some(Value(os("c"))));
}

// ------------------------------------------------------------ raw args

#[test]
fn raw_args_drains_everything() {
    let mut p = Parser::from_args(["-x", "a", "--", "-b", "c"]);
    assert_eq!(p.next().unwrap(), Some(Short('x')));
    let rest: Vec<_> = p.raw_args().unwrap().map(lossy).collect();
    assert_eq!(rest, vec!["a", "--", "-b", "c"]);
    assert_eq!(p.next().unwrap(), None);
}

#[test]
fn raw_args_after_double_dash_keeps_positional_mode() {
    let mut p = Parser::from_args(["--", "a", "-b"]);
    assert_eq!(p.next().unwrap(), Some(Value(os("a"))));
    let mut raw = p.raw_args().unwrap();
    assert_eq!(raw.peek(), Some(os("-b").as_os_str()));
    assert_eq!(raw.len(), 1);
    assert_eq!(raw.next(), Some(os("-b")));
    assert_eq!(raw.next(), None);
    assert_eq!(p.next().unwrap(), None);
}

#[test]
fn raw_args_partial_then_continue_lexing() {
    let mut p = Parser::from_args(["run", "a", "-b"]);
    assert_eq!(p.next().unwrap(), Some(Value(os("run"))));
    {
        let mut raw = p.raw_args().unwrap();
        assert_eq!(raw.next_if(|a| a == "zzz"), None);
        assert_eq!(raw.next_if(|a| a == "a"), Some(os("a")));
        assert_eq!(raw.as_slice(), &[os("-b")]);
    }
    assert_eq!(p.next().unwrap(), Some(Short('b')));
}

#[test]
fn raw_args_with_pending_attached_value_is_an_error() {
    let mut p = Parser::from_args(["--opt=x", "y"]);
    assert_eq!(p.next().unwrap(), Some(Long("opt")));
    let e = p.raw_args().expect_err("pending value");
    assert_eq!(
        e.to_string(),
        "unexpected argument for option '--opt': \"x\""
    );
    // The pending value was consumed by the failed call; lexing continues.
    assert_eq!(p.next().unwrap(), Some(Value(os("y"))));

    let mut p = Parser::from_args(["-ox"]);
    assert_eq!(p.next().unwrap(), Some(Short('o')));
    assert!(p.raw_args().is_err());
    assert_eq!(p.next().unwrap(), None);
}

#[test]
fn try_raw_args_with_pending_value_is_none_and_keeps_it() {
    let mut p = Parser::from_args(["-ox", "y"]);
    assert_eq!(p.next().unwrap(), Some(Short('o')));
    assert!(p.try_raw_args().is_none());
    assert_eq!(p.optional_value().unwrap(), Some(os("x")));
    let rest: Vec<_> = p.try_raw_args().unwrap().collect();
    assert_eq!(rest, vec![os("y")]);
}

#[test]
fn try_raw_args_at_end_of_cluster_is_some() {
    let mut p = Parser::from_args(["-ab", "c"]);
    assert_eq!(p.next().unwrap(), Some(Short('a')));
    assert_eq!(p.next().unwrap(), Some(Short('b')));
    let rest: Vec<_> = p.try_raw_args().unwrap().collect();
    assert_eq!(rest, vec![os("c")]);
}

// ---------------------------------------------------------- bin name

#[test]
fn from_iter_takes_bin_name() {
    let mut p = Parser::from_iter(["prog", "-a"]);
    assert_eq!(p.bin_name(), Some("prog"));
    assert_eq!(p.bin_name_os(), Some(os("prog").as_os_str()));
    assert_eq!(p.next().unwrap(), Some(Short('a')));
    assert_eq!(p.next().unwrap(), None);
}

#[test]
fn from_iter_with_nothing_has_no_bin_name() {
    let p = Parser::from_iter(Vec::<String>::new());
    assert_eq!(p.bin_name(), None);
    assert_eq!(p.bin_name_os(), None);
}

#[test]
fn from_args_has_no_bin_name() {
    let p = Parser::from_args(["-a"]);
    assert_eq!(p.bin_name(), None);
}

#[test]
fn from_args_accepts_owned_and_borrowed_types() {
    let _ = Parser::from_args(vec![String::from("a")]);
    let _ = Parser::from_args(vec![os("a")]);
    let _ = Parser::from_args(["a"]);
    let _ = Parser::from_args(std::env::args_os().skip(1));
}

// ----------------------------------------------------------- unexpected

#[test]
fn unexpected_renders_the_argument() {
    assert_eq!(Short('x').unexpected().to_string(), "invalid option '-x'");
    assert_eq!(
        Long("foo").unexpected().to_string(),
        "invalid option '--foo'"
    );
    assert_eq!(
        Value(os("bar")).unexpected().to_string(),
        "unexpected argument \"bar\""
    );
}

#[test]
fn unexpected_variants() {
    assert!(matches!(Short('x').unexpected(), Error::UnexpectedOption(o) if o == "-x"));
    assert!(matches!(Long("foo").unexpected(), Error::UnexpectedOption(o) if o == "--foo"));
    assert!(matches!(Value(os("b")).unexpected(), Error::UnexpectedArgument(v) if v == "b"));
}

// ------------------------------------------------------------- ValueExt

#[test]
fn value_ext_parse_and_string() {
    assert_eq!(os("42").parse::<u8>().unwrap(), 42);
    assert_eq!(os("hi").string().unwrap(), "hi");
    let e = os("300").parse::<u8>().unwrap_err();
    assert_eq!(
        e.to_string(),
        "cannot parse argument \"300\": number too large to fit in target type"
    );
    assert!(std::error::Error::source(&e).is_some());
    assert!(matches!(e, Error::ParsingFailed { value, .. } if value == "300"));
}

#[test]
fn value_ext_parse_with() {
    let v = os("a,b").parse_with(|s| -> Result<Vec<String>, &str> {
        Ok(s.split(',').map(str::to_owned).collect())
    });
    assert_eq!(v.unwrap(), vec!["a", "b"]);
    let e = os("x")
        .parse_with(|_| -> Result<(), &str> { Err("nope") })
        .unwrap_err();
    assert_eq!(e.to_string(), "cannot parse argument \"x\": nope");
}

#[test]
fn error_conversions() {
    let e: Error = "plain".into();
    assert_eq!(e.to_string(), "plain");
    let e: Error = String::from("owned").into();
    assert_eq!(e.to_string(), "owned");
    let boxed: Box<dyn std::error::Error + Send + Sync> = "boxed".into();
    let e: Error = boxed.into();
    assert_eq!(e.to_string(), "boxed");
    assert!(matches!(e, Error::Custom(_)));
}

#[test]
fn error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Error>();
    assert_send_sync::<Parser>();
}

// ------------------------------------------------- non-Unicode (unix only)

#[cfg(unix)]
mod non_unicode {
    use super::*;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    fn b(bytes: &[u8]) -> OsString {
        OsString::from_vec(bytes.to_vec())
    }

    #[test]
    fn positional_value_passes_through() {
        let mut p = Parser::from_args([b(b"\xff\xfe")]);
        assert_eq!(p.next().unwrap(), Some(Value(b(b"\xff\xfe"))));
    }

    #[test]
    fn long_attached_value_passes_through() {
        let mut p = Parser::from_args([b(b"--name=\xff")]);
        assert_eq!(p.next().unwrap(), Some(Long("name")));
        assert_eq!(p.value().unwrap(), b(b"\xff"));
    }

    #[test]
    fn long_separate_value_passes_through() {
        let mut p = Parser::from_args([os("--name"), b(b"\xff")]);
        assert_eq!(p.next().unwrap(), Some(Long("name")));
        assert_eq!(p.value().unwrap(), b(b"\xff"));
    }

    #[test]
    fn long_name_that_is_not_unicode_is_an_error() {
        let mut p = Parser::from_args([b(b"--na\xffme=v"), os("x")]);
        let e = p.next().unwrap_err();
        assert_eq!(e.to_string(), "invalid option '--na\u{FFFD}me'");
        assert!(matches!(e, Error::UnexpectedOption(_)));
        // Recovers: the whole bad argument is skipped.
        assert_eq!(p.next().unwrap(), Some(Value(os("x"))));
    }

    #[test]
    fn short_attached_value_passes_through() {
        let mut p = Parser::from_args([b(b"-n\xff\xfe")]);
        assert_eq!(p.next().unwrap(), Some(Short('n')));
        assert_eq!(p.value().unwrap(), b(b"\xff\xfe"));

        let mut p = Parser::from_args([b(b"-n=\xff")]);
        assert_eq!(p.next().unwrap(), Some(Short('n')));
        assert_eq!(p.value().unwrap(), b(b"\xff"));
    }

    #[test]
    fn invalid_byte_as_short_flag_becomes_replacement_char() {
        let mut p = Parser::from_args([b(b"-a\xffb")]);
        assert_eq!(p.next().unwrap(), Some(Short('a')));
        assert_eq!(p.next().unwrap(), Some(Short('\u{FFFD}')));
        assert_eq!(p.next().unwrap(), Some(Short('b')));
        assert_eq!(p.next().unwrap(), None);
    }

    #[test]
    fn value_after_invalid_short_flag_keeps_raw_bytes() {
        let mut p = Parser::from_args([b(b"-\xffo\xfe")]);
        assert_eq!(p.next().unwrap(), Some(Short('\u{FFFD}')));
        assert_eq!(p.next().unwrap(), Some(Short('o')));
        assert_eq!(p.value().unwrap(), b(b"\xfe"));
    }

    #[test]
    fn values_iter_passes_through() {
        let mut p = Parser::from_args([os("-e"), b(b"\xff"), os("ok")]);
        assert_eq!(p.next().unwrap(), Some(Short('e')));
        let got: Vec<_> = p.values().unwrap().collect();
        assert_eq!(got, vec![b(b"\xff"), os("ok")]);
    }

    #[test]
    fn string_conversion_fails_cleanly() {
        let e = b(b"\xff").string().unwrap_err();
        assert!(matches!(e, Error::NonUnicodeValue(v) if v.as_bytes() == b"\xff"));
        let e = b(b"\xff").parse::<u8>().unwrap_err();
        assert_eq!(e.to_string(), "argument is invalid unicode: \"\\xFF\"");
    }

    #[test]
    fn unexpected_value_error_carries_raw_bytes() {
        let mut p = Parser::from_args([b(b"--flag=\xff")]);
        assert_eq!(p.next().unwrap(), Some(Long("flag")));
        match p.next() {
            Err(Error::UnexpectedValue { option, value }) => {
                assert_eq!(option, "--flag");
                assert_eq!(value.as_bytes(), b"\xff");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn non_unicode_bin_name() {
        let p = Parser::from_iter([b(b"pr\xffog")]);
        assert_eq!(p.bin_name(), None);
        assert_eq!(p.bin_name_os().map(OsStr::as_bytes), Some(&b"pr\xffog"[..]));
    }

    use std::ffi::OsStr;
}

// -------------------------------------------------- spec example end-to-end

#[test]
fn spec_example_program() {
    fn run(args: &[&str]) -> Result<(u32, bool, String), Error> {
        let mut number = 1u32;
        let mut shout = false;
        let mut thing: Option<String> = None;
        let mut p = Parser::from_args(args.iter().copied());
        while let Some(arg) = p.next()? {
            match arg {
                Short('n') | Long("number") => number = p.value()?.parse()?,
                Long("shout") => shout = true,
                Value(v) => thing = Some(v.string()?),
                _ => return Err(arg.unexpected()),
            }
        }
        Ok((number, shout, thing.ok_or("missing argument THING")?))
    }

    assert_eq!(run(&["world"]).unwrap(), (1, false, "world".into()));
    assert_eq!(
        run(&["-n3", "--shout", "x"]).unwrap(),
        (3, true, "x".into())
    );
    assert_eq!(
        run(&["--number=2", "--", "-x"]).unwrap(),
        (2, false, "-x".into())
    );
    assert_eq!(run(&[]).unwrap_err().to_string(), "missing argument THING");
    assert_eq!(
        run(&["--nope", "x"]).unwrap_err().to_string(),
        "invalid option '--nope'"
    );
    assert_eq!(
        run(&["-n", "x"]).unwrap_err().to_string(),
        "cannot parse argument \"x\": invalid digit found in string"
    );
    assert_eq!(
        run(&["--shout=1"]).unwrap_err().to_string(),
        "unexpected argument for option '--shout': \"1\""
    );
}
