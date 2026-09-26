//! Shared exercise harness used by both the stable pseudo-fuzz test and the
//! `cargo fuzz` target. It drives a `Parser` with an arbitrary command line
//! and an arbitrary script of accessor calls, and checks invariants that must
//! hold for *any* input:
//!
//! 1. nothing panics (the harness itself is panic-free by construction, so any
//!    panic comes from the lexer);
//! 2. lexing terminates within a bound derived from the input size;
//! 3. once `next()` returns `Ok(None)` it keeps doing so;
//! 4. every value handed out is a byte-suffix of one of the input arguments
//!    (attached values are suffixes, separate values are whole arguments);
//! 5. long option names never contain `=`.

use std::ffi::OsString;

use kanna_core::{Arg, Parser};

/// Build an `OsString` from raw bytes. Non-Unicode bytes are only representable
/// on Unix; elsewhere the input is decoded lossily, which still exercises the
/// lexer with plenty of odd shapes.
pub fn os_from_bytes(bytes: &[u8]) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(bytes.to_vec())
    }
    #[cfg(not(unix))]
    {
        OsString::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

fn is_suffix_of_some(args: &[Vec<u8>], value: &OsString) -> bool {
    let v = value.as_encoded_bytes();
    args.iter().any(|a| {
        let a = os_from_bytes(a);
        a.as_encoded_bytes().ends_with(v)
    })
}

/// Run the parser over `args`, choosing an accessor after each token according
/// to `script` (cycled). Returns the number of tokens seen.
pub fn exercise(args: &[Vec<u8>], script: &[u8]) -> usize {
    let mut p = Parser::from_args(args.iter().map(|a| os_from_bytes(a)));
    let total_bytes: usize = args.iter().map(Vec::len).sum();
    // Every call to `next()` consumes at least one byte or one argument, or
    // returns an error that consumes pending state. Give generous slack.
    let bound = 4 * (total_bytes + args.len()) + 16;
    let mut steps = 0usize;
    let mut tokens = 0usize;
    let mut script_pos = 0usize;
    let mut pick = || {
        let b = script.get(script_pos).copied().unwrap_or(0);
        script_pos = (script_pos + 1) % script.len().max(1);
        b
    };

    let check = |v: &OsString| {
        assert!(
            is_suffix_of_some(args, v),
            "value {v:?} is not a suffix of any input argument"
        );
    };

    loop {
        steps += 1;
        assert!(
            steps <= bound,
            "lexer did not terminate within {bound} steps"
        );
        match p.next() {
            Ok(None) => break,
            Err(_) => continue,
            Ok(Some(arg)) => {
                tokens += 1;
                match &arg {
                    Arg::Long(name) => {
                        assert!(!name.contains('='), "long name {name:?} contains '='")
                    }
                    Arg::Short(_) => {}
                    Arg::Value(v) => check(v),
                }
            }
        }
        match pick() % 6 {
            0 => {}
            1 => {
                if let Ok(v) = p.value() {
                    check(&v);
                }
            }
            2 => {
                if let Ok(Some(v)) = p.optional_value() {
                    check(&v);
                }
            }
            3 => {
                if let Ok(it) = p.values() {
                    let vs: Vec<OsString> = it.collect();
                    assert!(!vs.is_empty(), "values() succeeded but yielded nothing");
                    vs.iter().for_each(check);
                }
            }
            4 => {
                let n = usize::from(pick() % 3);
                if let Ok(mut raw) = p.raw_args() {
                    for _ in 0..n {
                        match raw.next() {
                            Some(v) => check(&v),
                            None => break,
                        }
                    }
                }
            }
            _ => {
                if let Some(raw) = p.try_raw_args() {
                    let n = usize::from(pick() % 3);
                    raw.take(n).for_each(|v| check(&v));
                }
            }
        }
    }

    for _ in 0..3 {
        assert!(
            matches!(p.next(), Ok(None)),
            "next() after end must stay Ok(None)"
        );
    }
    tokens
}
