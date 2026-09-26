//! The sample CLI with bpaf's combinator API (no derive).
#![forbid(unsafe_code)]

use std::ffi::OsString;

use bpaf::{OptionParser, Parser, construct, long, positional, short};

#[derive(Debug, Clone)]
struct Args {
    verbose: bool,
    name: Option<String>,
    count: u32,
    output: Option<OsString>,
    color: Option<String>,
    input: OsString,
}

fn args() -> OptionParser<Args> {
    let verbose = short('v').long("verbose").help("Say more").switch();
    let name = short('n')
        .long("name")
        .help("Your name")
        .argument::<String>("NAME")
        .optional();
    let count = short('c')
        .long("count")
        .help("Repeat")
        .argument::<u32>("COUNT")
        .fallback(1);
    let output = short('o')
        .long("output")
        .help("Where to write")
        .argument::<OsString>("OUTPUT")
        .optional();
    let color = long("color")
        .help("When to colour")
        .argument::<String>("COLOR")
        .optional();
    let input = positional::<OsString>("INPUT").help("What to read");
    construct!(Args {
        verbose,
        name,
        count,
        output,
        color,
        input
    })
    .to_options()
    .descr("Size-gate sample")
    .version("0.1.0")
}

fn main() {
    let a = args().run();
    println!(
        "{} {:?} {} {:?} {:?} {:?}",
        a.verbose, a.name, a.count, a.output, a.color, a.input
    );
}
