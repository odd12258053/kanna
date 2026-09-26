//! Layer 3: the `cli!` macro. A struct definition, no proc-macro.
//! Run: `cargo run --example macro -- add cake --sweet`
#![forbid(unsafe_code)]

use hasami::Cli;

hasami::cli! {
    /// Keep a list of things
    #[derive(Debug)]
    #[name = "things", version = "0.1.0"]
    struct Args {
        /// Say what is happening
        #[short, global] verbose: bool,
        #[subcommand] cmd: Cmd,
    }

    #[derive(Debug)]
    enum Cmd {
        /// Add a thing
        Add(AddArgs),
        /// Remove things
        #[name = "rm", alias = "remove"]
        Remove(RemoveArgs),
        /// List everything
        List,
    }

    #[derive(Debug)]
    struct AddArgs {
        /// What to add
        #[positional] thing: String,
        /// Mark it as sweet
        sweet: bool,
        /// How many
        #[short, default = 1] count: u32,
    }

    #[derive(Debug)]
    struct RemoveArgs {
        /// What to remove
        #[positional, required] things: Vec<String>,
    }
}

fn main() {
    let args = Args::parse();
    if args.verbose {
        eprintln!("{args:#?}");
    }
    match args.cmd {
        Cmd::Add(a) => println!(
            "adding {} × {}{}",
            a.count,
            a.thing,
            if a.sweet { " (sweet)" } else { "" }
        ),
        Cmd::Remove(r) => println!("removing {}", r.things.join(", ")),
        Cmd::List => println!("(nothing yet)"),
    }
}
