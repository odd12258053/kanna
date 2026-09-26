//! The sample CLI written with `#[derive(Args)]`.
#![forbid(unsafe_code)]

use std::ffi::OsString;

use kanna::{Args, Cli};

/// Size-gate sample
#[derive(Args)]
#[kanna(name = "sample", version = "0.1.0")]
struct Sample {
    /// Say more
    #[kanna(short)]
    verbose: bool,
    /// Your name
    #[kanna(short)]
    name: Option<String>,
    /// Repeat
    #[kanna(short, default = 1)]
    count: u32,
    /// Where to write
    #[kanna(short)]
    output: Option<OsString>,
    /// When to colour
    #[kanna(default = String::from("auto"), default_missing = String::from("always"))]
    color: String,
    /// What to read
    #[kanna(positional)]
    input: OsString,
}

fn main() {
    let a = Sample::parse();
    println!(
        "{} {:?} {} {:?} {:?} {:?}",
        a.verbose, a.name, a.count, a.output, a.color, a.input
    );
}
