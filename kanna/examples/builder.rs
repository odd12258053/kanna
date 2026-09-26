//! Layer 2: the declarative builder. Typed values, help, constraints,
//! subcommands. Run: `cargo run --example builder -- --help`
#![forbid(unsafe_code)]

use kanna::{Arg, Command};

fn main() {
    let number = Arg::new("number")
        .short('n')
        .value::<u32>()
        .default(1)
        .help("How many times");
    let shout = Arg::new("shout").help("Use upper case");
    let quiet = Arg::new("quiet").short('q').help("Print nothing");
    let thing = Arg::positional::<String>("THING")
        .required()
        .help("Whom to greet");

    let cmd = Command::new("greet")
        .version("0.1.0")
        .about("Greet someone")
        .arg(&number)
        .arg(&shout)
        .arg(&quiet)
        .arg(&thing)
        .exclusive([shout.id(), quiet.id()]);

    let m = cmd.parse();
    if m.get(&quiet) {
        return;
    }
    let mut message = format!("Hello {}", m.get(&thing));
    if m.get(&shout) {
        message = message.to_uppercase();
    }
    for _ in 0..m.get(&number) {
        println!("{message}");
    }
}
