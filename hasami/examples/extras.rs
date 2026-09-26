//! The generator crates: shell completion (static and dynamic), manpage /
//! Markdown / HTML, and the JSON schema, all from one `Command`.
//!
//! Run: `cargo run --example extras -- completions bash`
//!      `cargo run --example extras -- doc markdown`
//!      `cargo run --example extras -- schema`
//!      `_HASAMI_COMPLETE=bash _HASAMI_COMPLETE_WORDS=$'extras\x1fdo\x1f--br' _HASAMI_COMPLETE_INDEX=2 cargo run -q --example extras`
#![forbid(unsafe_code)]

use hasami::{Arg, Command};
use hasami_complete::Shell;

fn command() -> Command {
    let branch = Arg::new("branch")
        .short('b')
        .value::<String>()
        .help("Branch to work on")
        .complete_with(|prefix| {
            ["main", "develop", "feature/login"]
                .iter()
                .filter(|b| b.starts_with(prefix))
                .map(|b| (*b).to_owned())
                .collect()
        });
    let dry = Arg::new("dry-run").help("Do nothing");
    let shell = Arg::positional::<Shell>("SHELL")
        .possible(["bash", "zsh", "fish", "powershell", "nushell"])
        .required();
    let dynamic = Arg::new("dynamic").help("Delegate completion to the binary at runtime");
    let format = Arg::positional::<String>("FORMAT")
        .possible(["man", "markdown", "html"])
        .required();
    Command::new("extras")
        .version("0.1.0")
        .about("Show the generator crates")
        .subcommand(
            Command::new("do")
                .about("Do the thing")
                .arg(&branch)
                .arg(&dry),
        )
        .subcommand(
            Command::new("completions")
                .about("Print a completion script")
                .arg(&shell)
                .arg(&dynamic),
        )
        .subcommand(
            Command::new("doc")
                .about("Print documentation")
                .arg(&format),
        )
        .subcommand(Command::new("schema").about("Print the JSON description"))
        .subcommand_required()
}

fn main() {
    let cmd = command();
    // Answer completion requests before parsing.
    hasami_complete::dynamic::complete_from_env(&cmd);
    let m = cmd.parse();
    match m.subcommand() {
        Some(("do", sm)) => println!(
            "doing on {:?}, dry={}",
            sm.raw_id("branch"),
            sm.contains_id("dry-run")
        ),
        Some(("completions", sm)) => {
            let shell: Shell = sm.raw_id("SHELL")[0]
                .to_string_lossy()
                .parse()
                .unwrap_or(Shell::Bash);
            if sm.contains_id("dynamic") {
                print!("{}", hasami_complete::generate_dynamic(shell, "extras"));
            } else {
                print!("{}", hasami_complete::generate(shell, &cmd));
            }
        }
        Some(("doc", sm)) => {
            let text = match sm.raw_id("FORMAT")[0].to_str() {
                Some("man") => hasami_doc::manpage(&cmd),
                Some("html") => hasami_doc::html(&cmd),
                _ => hasami_doc::markdown(&cmd),
            };
            print!("{text}");
        }
        Some(("schema", _)) => print!("{}", hasami_schema::to_json_pretty(&cmd)),
        _ => {}
    }
}
