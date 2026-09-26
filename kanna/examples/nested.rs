//! Nested subcommands with the builder: global options, aliases, hidden
//! and lazily built subcommands, and how to dispatch on the result.
//!
//! Run: `cargo run --example nested -- remote add origin https://example.com/repo.git`
//!      `cargo run --example nested -- -v remote ls`
//!      `cargo run --example nested -- remote --help`
#![forbid(unsafe_code)]

use kanna::{Arg, Command, Matches, Subcommand};

/// `remote add <NAME> <URL>`, `remote rm <NAME>`, `remote ls`
fn remote() -> Command {
    let name = Arg::positional::<String>("NAME")
        .required()
        .help("Remote name");
    let url = Arg::positional::<String>("URL")
        .required()
        .help("Where it lives");
    let fetch = Arg::new("fetch")
        .short('f')
        .help("Fetch right after adding");

    let rm_name = name.clone();
    Command::new("remote")
        .about("Manage the set of tracked repositories")
        .subcommand(
            Command::new("add")
                .about("Add a remote")
                .arg(&name)
                .arg(&url)
                .arg(&fetch),
        )
        // A lazily built subcommand costs nothing until it is selected.
        .subcommand(
            Subcommand::lazy("rm", move || Command::new("rm").arg(&rm_name))
                .about("Remove a remote")
                .alias("remove"),
        )
        .subcommand(Subcommand::from(Command::new("ls").about("List remotes")).alias("list"))
        .subcommand_required()
}

fn main() {
    let verbose = Arg::new("verbose")
        .short('v')
        .global()
        .help("Explain what is being done");
    let all = Arg::new("all").short('a').help("Include hidden entries");
    let message = Arg::new("message")
        .short('m')
        .value::<String>()
        .many()
        .help("Commit message (repeat for more paragraphs)");

    let cmd = Command::new("vcs")
        .version("0.1.0")
        .about("A tiny version control front end")
        .arg(&verbose)
        .subcommand(
            Command::new("status")
                .about("Show the working tree")
                .arg(&all),
        )
        .subcommand(Command::new("commit").about("Record changes").arg(&message))
        .subcommand(remote())
        // Accepted but not listed in help: handy for maintenance commands.
        .subcommand(Subcommand::from(Command::new("gc").about("Collect garbage")).hidden())
        .subcommand_required();

    let m = cmd.parse();
    // A global option can be read at the top level wherever it was written:
    // `vcs -v remote ls` and `vcs remote ls -v` both set it.
    let verbose = m.get(&verbose);
    match m.subcommand() {
        Some(("status", sm)) => println!("status (all = {})", sm.get(&all)),
        Some(("commit", sm)) => {
            let paragraphs = sm.get(&message);
            if paragraphs.is_empty() {
                println!("commit (an editor would open)");
            } else {
                println!("commit: {}", paragraphs.join("\n\n"));
            }
        }
        Some(("remote", sm)) => remote_main(sm, verbose),
        Some(("gc", _)) => println!("collecting garbage"),
        _ => unreachable!("subcommand_required() rejects the empty case"),
    }
}

/// The nested level has its own `Matches`; positionals are read by id
/// because the `Arg` values live in `remote()`.
fn remote_main(m: &Matches, verbose: bool) {
    match m.subcommand() {
        Some(("add", sm)) => {
            let name = sm.raw_id("NAME")[0].to_string_lossy();
            let url = sm.raw_id("URL")[0].to_string_lossy();
            println!("adding remote {name} -> {url}");
            if sm.contains_id("fetch") {
                println!("fetching {name}");
            }
        }
        // Aliases resolve to the canonical name.
        Some(("rm", sm)) => println!("removing remote {}", sm.raw_id("NAME")[0].to_string_lossy()),
        Some(("ls", _)) => {
            println!("origin");
            if verbose {
                println!("  https://example.com/repo.git (fetch)");
            }
        }
        _ => unreachable!(),
    }
}
