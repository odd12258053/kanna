//! The sample CLI written with the `cli!` macro.
#![forbid(unsafe_code)]

use std::ffi::OsString;

hasami::cli! {
    /// Size-gate sample
    #[name = "sample", version = "0.1.0"]
    struct Args {
        /// Say more
        #[short] verbose: bool,
        /// Your name
        #[short] name: Option<String>,
        /// Repeat
        #[short, default = 1] count: u32,
        /// Where to write
        #[short] output: Option<OsString>,
        /// When to colour
        #[default = String::from("auto"), default_missing = String::from("always")]
        color: String,
        /// What to read
        #[positional] input: OsString,
    }
}

fn main() {
    use hasami::Cli;
    let a = Args::parse();
    println!(
        "{} {:?} {} {:?} {:?} {:?}",
        a.verbose, a.name, a.count, a.output, a.color, a.input
    );
}
