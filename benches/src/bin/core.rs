//! The sample CLI written against `hasami-core` alone.
#![forbid(unsafe_code)]

use hasami_core::prelude::*;

fn run() -> Result<(), hasami_core::Error> {
    let mut verbose = false;
    let mut name: Option<String> = None;
    let mut count = 1u32;
    let mut output: Option<std::ffi::OsString> = None;
    let mut color: Option<String> = None;
    let mut input: Option<std::ffi::OsString> = None;

    let mut p = Parser::from_env();
    while let Some(arg) = p.next()? {
        match arg {
            Short('v') | Long("verbose") => verbose = true,
            Short('n') | Long("name") => name = Some(p.value()?.string()?),
            Short('c') | Long("count") => count = p.value()?.parse()?,
            Short('o') | Long("output") => output = Some(p.value()?),
            Long("color") => {
                color = Some(
                    p.optional_value()?
                        .map_or(Ok("auto".into()), |v| v.string())?,
                )
            }
            Value(v) if input.is_none() => input = Some(v),
            _ => return Err(arg.unexpected()),
        }
    }
    let input = input.ok_or("missing argument INPUT")?;
    println!("{verbose} {name:?} {count} {output:?} {color:?} {input:?}");
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
}
