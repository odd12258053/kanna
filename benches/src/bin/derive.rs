//! The sample CLI written with `#[derive(Args)]`.
#![forbid(unsafe_code)]

use std::ffi::OsString;

use hasami::{Args, Cli};

/// Size-gate sample
#[derive(Args)]
#[hasami(name = "sample", version = "0.1.0")]
struct Sample {
    /// Say more
    #[hasami(short)]
    verbose: bool,
    /// Your name
    #[hasami(short)]
    name: Option<String>,
    /// Repeat
    #[hasami(short, default = 1)]
    count: u32,
    /// Where to write
    #[hasami(short)]
    output: Option<OsString>,
    /// When to colour
    #[hasami(default = String::from("auto"), default_missing = String::from("always"))]
    color: String,
    /// What to read
    #[hasami(positional)]
    input: OsString,
}

fn main() {
    let a = Sample::parse();
    println!(
        "{} {:?} {} {:?} {:?} {:?}",
        a.verbose, a.name, a.count, a.output, a.color, a.input
    );
}
