//! Layer 4: `#[derive(Args)]`, feature `derive`. Same semantics as `cli!`.
//! Run: `cargo run --features derive --example derive -- --help`
#![forbid(unsafe_code)]

use std::path::PathBuf;

use kanna::{Args, Cli, Commands};

/// Convert files
#[derive(Args, Debug)]
#[kanna(name = "convert", version = "0.1.0")]
struct Convert {
    /// Output directory
    #[kanna(short, long = "out-dir", value_name = "DIR")]
    output: Option<PathBuf>,
    /// Colour output
    #[kanna(default = String::from("auto"), default_missing = String::from("always"), possible = ["auto", "always", "never"])]
    color: String,
    /// More output (repeatable)
    #[kanna(short, count)]
    verbose: usize,
    #[kanna(subcommand)]
    format: Format,
}

#[derive(Commands, Debug)]
enum Format {
    /// Convert to PNG
    Png(Png),
    /// Convert to JPEG
    #[kanna(alias = "jpg")]
    Jpeg(Jpeg),
}

#[derive(Args, Debug)]
struct Png {
    /// Input files
    #[kanna(positional, required)]
    files: Vec<PathBuf>,
}

#[derive(Args, Debug)]
struct Jpeg {
    /// Quality, 1-100
    #[kanna(short, default = 85)]
    quality: u8,
    /// Input files
    #[kanna(positional, required)]
    files: Vec<PathBuf>,
}

fn main() {
    let args = Convert::parse();
    let files = match &args.format {
        Format::Png(p) => &p.files,
        Format::Jpeg(j) => {
            println!("quality {}", j.quality);
            &j.files
        }
    };
    println!(
        "converting {} file(s) to {:?}, colour={}, verbosity={}",
        files.len(),
        args.output,
        args.color,
        args.verbose
    );
}
