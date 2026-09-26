//! The generator crates: shell completion (static and dynamic), manpage /
//! Markdown / HTML, and the JSON schema, all from one `Command`.
//!
//! Run: `cargo run --example extras -- completions bash`
//!      `cargo run --example extras -- doc markdown`
//!      `cargo run --example extras -- schema`
//!      `cargo run --example extras -- tools`
//!      `cargo run --example extras -- tools --call extras_do '{"branch": "main", "dry-run": true}'`
//!      `_KANNA_COMPLETE=bash _KANNA_COMPLETE_WORDS=$'extras\x1fdo\x1f--br' _KANNA_COMPLETE_INDEX=2 cargo run -q --example extras`
#![forbid(unsafe_code)]

use kanna::{Arg, Command};
use kanna_complete::Shell;

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
    let call = Arg::new("call")
        .value::<String>()
        .value_name("TOOL")
        .help("Convert a tool call to a command line instead of printing definitions");
    let input = Arg::positional::<String>("INPUT")
        .help("The tool call's JSON input (with --call)")
        .requires(&call);
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
        .subcommand(
            Command::new("tools")
                .about("Print tool definitions for an AI agent, or turn a tool call into a command line")
                .arg(&call)
                .arg(&input)
                .example("extras tools")
                .example("extras tools --call extras_do '{\"branch\": \"main\"}'"),
        )
        .subcommand_required()
}

fn main() {
    let cmd = command();
    // Answer completion requests before parsing.
    kanna_complete::dynamic::complete_from_env(&cmd);
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
                print!("{}", kanna_complete::generate_dynamic(shell, "extras"));
            } else {
                print!("{}", kanna_complete::generate(shell, &cmd));
            }
        }
        Some(("doc", sm)) => {
            let text = match sm.raw_id("FORMAT")[0].to_str() {
                Some("man") => kanna_doc::manpage(&cmd),
                Some("html") => kanna_doc::html(&cmd),
                _ => kanna_doc::markdown(&cmd),
            };
            print!("{text}");
        }
        Some(("schema", _)) => print!("{}", kanna_schema::to_json_pretty(&cmd)),
        Some(("tools", sm)) => {
            let call = sm
                .raw_id("call")
                .first()
                .map(|v| v.to_string_lossy().into_owned());
            match call {
                None => println!(
                    "{}",
                    kanna_schema::tool::to_json(&kanna_schema::tool::tools(&cmd), false)
                ),
                Some(tool) => {
                    let input = sm
                        .raw_id("INPUT")
                        .first()
                        .map(|v| v.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "{}".to_owned());
                    match kanna_schema::tool::to_argv(&cmd, &tool, &input) {
                        Ok(argv) => println!("{argv:?}"),
                        Err(e) => {
                            eprintln!("error: {e}");
                            std::process::exit(2);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
