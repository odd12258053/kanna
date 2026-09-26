//! Layer 1: the imperative lexer, `hasami-core`. No definitions, no help,
//! just a loop. Run: `cargo run --example core -- -n 2 --shout world`
#![forbid(unsafe_code)]

use hasami::lex::prelude::*;

fn run() -> Result<(), hasami::lex::Error> {
    let mut number = 1u32;
    let mut shout = false;
    let mut thing: Option<String> = None;

    let mut parser = Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('n') | Long("number") => number = parser.value()?.parse()?,
            Long("shout") => shout = true,
            Value(v) if thing.is_none() => thing = Some(v.string()?),
            _ => return Err(arg.unexpected()),
        }
    }

    let thing = thing.ok_or("missing argument THING")?;
    let mut message = format!("Hello {thing}");
    if shout {
        message = message.to_uppercase();
    }
    for _ in 0..number {
        println!("{message}");
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
}
