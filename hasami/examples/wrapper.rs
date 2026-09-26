//! Layer 1 beyond the basics: a wrapper that launches another program.
//! Shows optional values (`--color[=WHEN]`), multi-value options
//! (`--env A=1 B=2`), the raw tail after `--`, non-Unicode arguments and
//! collecting several errors before giving up.
//!
//! Run: `cargo run --example wrapper -- -n2 --env A=1 B=2 --color -- echo hello world`
//!      `cargo run --example wrapper -- --bogus --color=sometimes -- ls`   (two errors)
#![forbid(unsafe_code)]

use std::ffi::OsString;

use hasami::lex::prelude::*;

#[derive(Debug)]
struct Options {
    times: u32,
    color: Color,
    env: Vec<(String, String)>,
    command: Vec<OsString>,
}

#[derive(Debug, Clone, Copy)]
enum Color {
    Auto,
    Always,
    Never,
}

fn parse_color(s: &str) -> Result<Color, String> {
    match s {
        "auto" => Ok(Color::Auto),
        "always" => Ok(Color::Always),
        "never" => Ok(Color::Never),
        _ => Err(format!("expected auto, always or never, got '{s}'")),
    }
}

fn parse_assignment(v: OsString) -> Result<(String, String), hasami::lex::Error> {
    let s = v.string()?;
    match s.split_once('=') {
        Some((k, v)) if !k.is_empty() => Ok((k.to_owned(), v.to_owned())),
        _ => Err(format!("expected KEY=VALUE, got '{s}'").into()),
    }
}

fn parse(parser: &mut Parser) -> Result<Options, Vec<hasami::lex::Error>> {
    let mut opts = Options {
        times: 1,
        color: Color::Auto,
        env: Vec::new(),
        command: Vec::new(),
    };
    let mut errors = Vec::new();

    loop {
        // Every error leaves the parser in a consistent state, so the loop
        // can keep going and report all problems at once.
        let arg = match parser.next() {
            Ok(Some(arg)) => arg,
            Ok(None) => break,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        let step = match arg {
            Short('n') | Long("times") => parser
                .value()
                .and_then(|v| v.parse().map(|n| opts.times = n)),
            // `--color` alone is "always"; `--color=never` attaches a value.
            // `--color never` would leave `never` as a positional.
            Long("color") => parser.optional_value().and_then(|v| match v {
                None => {
                    opts.color = Color::Always;
                    Ok(())
                }
                Some(v) => v.parse_with(parse_color).map(|c| opts.color = c),
            }),
            // `--env A=1 B=2` takes every following non-option argument.
            Short('e') | Long("env") => parser.values().and_then(|values| {
                for v in values {
                    opts.env.push(parse_assignment(v)?);
                }
                Ok(())
            }),
            // The first positional starts the command; the rest is taken
            // verbatim, options included: `wrapper -- ls -la` runs `ls -la`.
            Value(program) => {
                opts.command.push(program);
                match parser.raw_args() {
                    Ok(rest) => {
                        opts.command.extend(rest);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            other => Err(other.unexpected()),
        };
        if let Err(e) = step {
            errors.push(e);
        }
    }

    if opts.command.is_empty() {
        errors.push("missing command to run (put it after '--')".into());
    }
    if errors.is_empty() {
        Ok(opts)
    } else {
        Err(errors)
    }
}

fn main() {
    let mut parser = Parser::from_env();
    let opts = match parse(&mut parser) {
        Ok(opts) => opts,
        Err(errors) => {
            for e in &errors {
                eprintln!("error: {e}");
            }
            let bin = parser.bin_name().unwrap_or("wrapper");
            eprintln!(
                "usage: {bin} [-n N] [--color[=WHEN]] [--env KEY=VALUE...] -- COMMAND [ARGS...]"
            );
            std::process::exit(2);
        }
    };

    let (program, args) = opts.command.split_first().expect("checked in parse");
    for _ in 0..opts.times {
        // Arguments are `OsString`s all the way to the child process, so a
        // file name that is not valid Unicode arrives intact.
        let status = std::process::Command::new(program)
            .args(args)
            .envs(opts.env.iter().map(|(k, v)| (k, v)))
            .env(
                "CLICOLOR_FORCE",
                matches!(opts.color, Color::Always)
                    .then_some("1")
                    .unwrap_or("0"),
            )
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => std::process::exit(s.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("error: cannot run {}: {e}", program.to_string_lossy());
                std::process::exit(127);
            }
        }
    }
}
