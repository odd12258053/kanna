//! The sample CLI written with the declarative builder.
#![forbid(unsafe_code)]

use std::ffi::OsString;

use hasami::{Arg, Command};

fn main() {
    let verbose = Arg::new("verbose").short('v').help("Say more");
    let name = Arg::new("name")
        .short('n')
        .value::<String>()
        .help("Your name");
    let count = Arg::new("count")
        .short('c')
        .value::<u32>()
        .default(1)
        .help("Repeat");
    let output = Arg::new("output")
        .short('o')
        .value_os()
        .help("Where to write");
    let color = Arg::new("color")
        .value::<String>()
        .default_missing("always".to_owned())
        .default("auto".to_owned())
        .help("When to colour");
    let input = Arg::positional_os("INPUT").required().help("What to read");
    let cmd = Command::new("sample")
        .version("0.1.0")
        .about("Size-gate sample")
        .arg(&verbose)
        .arg(&name)
        .arg(&count)
        .arg(&output)
        .arg(&color)
        .arg(&input);
    let m = cmd.parse();
    let verbose = m.get(&verbose);
    let name: Option<String> = m.get(&name);
    let count = m.get(&count);
    let output: Option<OsString> = m.get(&output);
    let color = m.get(&color);
    let input = m.get(&input);
    println!("{verbose} {name:?} {count} {output:?} {color:?} {input:?}");
}
