//! The sample CLI with pico-args.
#![forbid(unsafe_code)]

use std::ffi::OsString;

fn run() -> Result<(), pico_args::Error> {
    let mut p = pico_args::Arguments::from_env();
    let verbose = p.contains(["-v", "--verbose"]);
    let name: Option<String> = p.opt_value_from_str(["-n", "--name"])?;
    let count: u32 = p.opt_value_from_str(["-c", "--count"])?.unwrap_or(1);
    let output: Option<OsString> =
        p.opt_value_from_os_str(["-o", "--output"], |s| Ok::<_, String>(s.to_os_string()))?;
    let color: Option<String> = p.opt_value_from_str("--color")?;
    let input: OsString = p.free_from_os_str(|s| Ok::<_, String>(s.to_os_string()))?;
    let rest = p.finish();
    if !rest.is_empty() {
        eprintln!("error: unexpected arguments {rest:?}");
        std::process::exit(2);
    }
    println!("{verbose} {name:?} {count} {output:?} {color:?} {input:?}");
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
}
