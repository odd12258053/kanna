//! The sample CLI with argh (derive). argh cannot take non-UTF-8 values.
#![forbid(unsafe_code)]

use argh::FromArgs;

/// Size-gate sample
#[derive(FromArgs)]
struct Args {
    /// say more
    #[argh(switch, short = 'v')]
    verbose: bool,
    /// your name
    #[argh(option, short = 'n')]
    name: Option<String>,
    /// repeat
    #[argh(option, short = 'c', default = "1")]
    count: u32,
    /// where to write
    #[argh(option, short = 'o')]
    output: Option<String>,
    /// when to colour
    #[argh(option)]
    color: Option<String>,
    /// what to read
    #[argh(positional)]
    input: String,
}

fn main() {
    let a: Args = argh::from_env();
    println!(
        "{} {:?} {} {:?} {:?} {:?}",
        a.verbose, a.name, a.count, a.output, a.color, a.input
    );
}
