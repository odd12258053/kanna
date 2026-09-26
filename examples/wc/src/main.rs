//! `wc`: count lines, words, bytes and characters, on `kanna-core` alone.
//!
//! The lexer knows nothing about the options, so `--help`, `--version`,
//! the usage text and the "unknown option" message are all written here.
//! In exchange the binary depends on nothing but `std`.
//!
//! ```text
//! cargo run -p kanna-example-wc -- -lw src/main.rs Cargo.toml
//! echo hello world | cargo run -p kanna-example-wc
//! ```
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::process::ExitCode;

use kanna_core::prelude::*;

const USAGE: &str = "Usage: wc [OPTION]... [FILE]...";
const HELP: &str = "\
Print newline, word and byte counts for each FILE, and a total line if
more than one FILE is specified. With no FILE, or when FILE is -, read
standard input.

Options:
  -c, --bytes            print the byte counts
  -m, --chars            print the character counts
  -l, --lines            print the newline counts
  -L, --max-line-length  print the maximum display width
  -w, --words            print the word counts
  -h, --help             display this help and exit
  -V, --version          output version information and exit";

#[derive(Default)]
struct Options {
    lines: bool,
    words: bool,
    bytes: bool,
    chars: bool,
    max_line: bool,
    files: Vec<OsString>,
}

/// What `main` should do once the arguments are understood.
enum Action {
    Run(Options),
    Help,
    Version,
}

fn parse_args(parser: &mut Parser) -> Result<Action, kanna_core::Error> {
    let mut o = Options::default();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('l') | Long("lines") => o.lines = true,
            Short('w') | Long("words") => o.words = true,
            Short('c') | Long("bytes") => o.bytes = true,
            Short('m') | Long("chars") => o.chars = true,
            Short('L') | Long("max-line-length") => o.max_line = true,
            Short('h') | Long("help") => return Ok(Action::Help),
            Short('V') | Long("version") => return Ok(Action::Version),
            // File names stay `OsString`: a name that is not valid UTF-8 is
            // still opened correctly.
            Value(path) => o.files.push(path),
            other => return Err(other.unexpected()),
        }
    }
    if !(o.lines || o.words || o.bytes || o.chars || o.max_line) {
        o.lines = true;
        o.words = true;
        o.bytes = true;
    }
    Ok(Action::Run(o))
}

#[derive(Default, Clone, Copy)]
struct Counts {
    lines: u64,
    words: u64,
    bytes: u64,
    chars: u64,
    max_line: u64,
}

impl Counts {
    fn add(&mut self, other: Counts) {
        self.lines += other.lines;
        self.words += other.words;
        self.bytes += other.bytes;
        self.chars += other.chars;
        self.max_line = self.max_line.max(other.max_line);
    }
}

fn count(reader: impl Read) -> io::Result<Counts> {
    let mut reader = BufReader::new(reader);
    let mut c = Counts::default();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let n = reader.read_until(b'\n', &mut buf)?;
        if n == 0 {
            break;
        }
        c.bytes += n as u64;
        let line = String::from_utf8_lossy(&buf);
        if line.ends_with('\n') {
            c.lines += 1;
        }
        let text = line.trim_end_matches('\n');
        c.chars += line.chars().count() as u64;
        c.words += text.split_whitespace().count() as u64;
        c.max_line = c.max_line.max(text.chars().count() as u64);
    }
    Ok(c)
}

fn print_counts(o: &Options, c: Counts, name: &str) {
    let mut cols = Vec::new();
    if o.lines {
        cols.push(c.lines);
    }
    if o.words {
        cols.push(c.words);
    }
    if o.chars {
        cols.push(c.chars);
    }
    if o.bytes {
        cols.push(c.bytes);
    }
    if o.max_line {
        cols.push(c.max_line);
    }
    let row: Vec<String> = cols.iter().map(|n| format!("{n:>7}")).collect();
    if name.is_empty() {
        println!("{}", row.join(" "));
    } else {
        println!("{} {name}", row.join(" "));
    }
}

fn run(o: &Options) -> ExitCode {
    let mut status = ExitCode::SUCCESS;
    let mut total = Counts::default();
    if o.files.is_empty() {
        return match count(io::stdin().lock()) {
            Ok(c) => {
                print_counts(o, c, "");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("wc: standard input: {e}");
                ExitCode::FAILURE
            }
        };
    }
    for path in &o.files {
        let name = path.to_string_lossy();
        let result = if path == "-" {
            count(io::stdin().lock())
        } else {
            File::open(path).and_then(count)
        };
        match result {
            Ok(c) => {
                print_counts(o, c, &name);
                total.add(c);
            }
            Err(e) => {
                eprintln!("wc: {name}: {e}");
                status = ExitCode::FAILURE;
            }
        }
    }
    if o.files.len() > 1 {
        print_counts(o, total, "total");
    }
    status
}

fn main() -> ExitCode {
    let mut parser = Parser::from_env();
    match parse_args(&mut parser) {
        Ok(Action::Run(o)) => run(&o),
        Ok(Action::Help) => {
            println!("{USAGE}\n{HELP}");
            ExitCode::SUCCESS
        }
        Ok(Action::Version) => {
            println!("wc (kanna example) {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("wc: {e}\n{USAGE}\nTry 'wc --help' for more information.");
            ExitCode::from(2)
        }
    }
}
