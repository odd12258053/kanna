//! Typed values with the builder: value enums, `possible` values, custom
//! parsers and validators, defaults for types without `Display`, repeated
//! options with delimiters, counters, optional values and raw `OsString`s.
//!
//! Run: `cargo run --example values -- -vv --level warn --timeout 2m30s -I lib -I vendor src`
//!      `cargo run --example values -- --color=never --retries 99 src`   (fails)
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::time::Duration;

use hasami::{Arg, Command};

// A value type of its own. `value_enum!` writes the `ValueEnum`, `FromStr`
// and `Display` impls from one list, so the names shown in help and the
// names accepted cannot drift apart.
hasami::value_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Level { Error = "error", Warn = "warn", Info = "info" }
}

/// `2m30s`, `45s`, `1h`. The error text ends up in the parse error.
fn parse_duration(s: &str) -> Result<Duration, String> {
    let mut total = Duration::ZERO;
    let mut digits = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        let n: u64 = digits
            .parse()
            .map_err(|_| format!("expected a number before '{c}'"))?;
        digits.clear();
        total += match c {
            'h' => Duration::from_secs(n * 3600),
            'm' => Duration::from_secs(n * 60),
            's' => Duration::from_secs(n),
            _ => return Err(format!("unknown unit '{c}' (use h, m or s)")),
        };
    }
    if !digits.is_empty() {
        return Err("trailing number without a unit".to_owned());
    }
    Ok(total)
}

/// Parse and validate in one step: the range is part of the type's contract.
fn parse_retries(s: &str) -> Result<u8, String> {
    let n: u8 = s.parse().map_err(|e| format!("{e}"))?;
    if (1..=10).contains(&n) {
        Ok(n)
    } else {
        Err(format!("{n} is out of range 1..=10"))
    }
}

fn main() {
    let level = Arg::new("level")
        .short('l')
        .value_enum::<Level>()
        .default(Level::Warn)
        .help("Minimum severity to report");
    // `Duration` has no `Display`, so the help text is given separately.
    let timeout = Arg::new("timeout")
        .short('t')
        .value_with(parse_duration)
        .default_with(Duration::from_secs(30), "30s")
        .help("Give up after this long");
    let retries = Arg::new("retries")
        .value_with(parse_retries)
        .default(3)
        .help("How often to retry");
    // `--color` alone means "always"; `--color=never` overrides. The value
    // must be attached with `=`, so `--color never` treats `never` as a path.
    let color = Arg::new("color")
        .value::<String>()
        .possible(["auto", "always", "never"])
        .default_missing("always".to_owned())
        .default("auto".to_owned())
        .help("When to use colour");
    // `-I a -I b` and `-I a:b` both give two directories.
    let include = Arg::new("include")
        .short('I')
        .value::<String>()
        .value_name("DIR")
        .many()
        .delimiter(':')
        .help("Extra directory to search (repeatable, or ':'-separated)");
    let verbose = Arg::new("verbose")
        .short('v')
        .count()
        .help("More output (-v, -vv, -vvv)");
    // Paths may contain bytes that are not valid Unicode; keep them raw.
    let paths = Arg::positional_os("PATH")
        .many()
        .required()
        .help("Files to check");

    let cmd = Command::new("check")
        .version("0.1.0")
        .about("Show how values are parsed")
        .arg(&level)
        .arg(&timeout)
        .arg(&retries)
        .arg(&color)
        .arg(&include)
        .arg(&verbose)
        .arg(&paths);

    let m = cmd.parse();

    println!("level    = {}", m.get(&level));
    println!("timeout  = {:?}", m.get(&timeout));
    println!("retries  = {}", m.get(&retries));
    println!("color    = {}", m.get(&color));
    println!("include  = {:?}", m.get(&include));
    println!("verbose  = {}", m.get(&verbose));
    let paths: Vec<OsString> = m.get(&paths);
    for p in &paths {
        println!("path     = {}", p.to_string_lossy());
    }
}
