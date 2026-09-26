//! The sample CLI with clap's builder (help + usage + error-context features).
#![forbid(unsafe_code)]

use std::ffi::OsString;

use clap::{Arg, ArgAction, Command};

fn main() {
    let cmd = Command::new("sample")
        .version("0.1.0")
        .about("Size-gate sample")
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::SetTrue)
                .help("Say more"),
        )
        .arg(Arg::new("name").short('n').long("name").help("Your name"))
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .value_parser(clap::value_parser!(u32))
                .default_value("1")
                .help("Repeat"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_parser(clap::value_parser!(OsString))
                .help("Where to write"),
        )
        .arg(
            Arg::new("color")
                .long("color")
                .num_args(0..=1)
                .default_value("auto")
                .default_missing_value("always")
                .help("When to colour"),
        )
        .arg(
            Arg::new("input")
                .required(true)
                .value_parser(clap::value_parser!(OsString))
                .help("What to read"),
        );
    let m = cmd.get_matches();
    let verbose = m.get_flag("verbose");
    let name = m.get_one::<String>("name");
    let count = m.get_one::<u32>("count").copied().unwrap_or(1);
    let output = m.get_one::<OsString>("output");
    let color = m.get_one::<String>("color");
    let input = m.get_one::<OsString>("input");
    println!("{verbose} {name:?} {count} {output:?} {color:?} {input:?}");
}
