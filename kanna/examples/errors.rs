//! Error handling with the builder: `try_parse` instead of `parse`,
//! inspecting `ErrorKind`, constraints (`requires`, groups), and reporting
//! application errors through the same `Error` type so they look the same.
//!
//! Run: `cargo run --example errors -- --from a.txt --to b.txt`
//!      `cargo run --example errors -- --from a.txt --to a.txt`        (custom error)
//!      `cargo run --example errors -- --from a.txt --to b.txt --force --dry-run` (conflict)
//!      `cargo run --example errors -- --frm a.txt`                    (unknown option)
#![forbid(unsafe_code)]

use std::path::PathBuf;

use kanna::{Arg, Command, Error, ErrorKind, Group};

fn run() -> Result<(), Error> {
    let from = Arg::new("from").value::<PathBuf>().help("Source file");
    let to = Arg::new("to").value::<PathBuf>().help("Destination file");
    let stdin = Arg::new("stdin").help("Read the source from standard input");
    let force = Arg::new("force")
        .short('f')
        .help("Overwrite the destination");
    let dry_run = Arg::new("dry-run")
        .short('n')
        .help("Only say what would happen");

    let cmd = Command::new("cp2")
        .version("0.1.0")
        .about("Copy one file, with all the ways it can go wrong")
        .arg(&from)
        .arg(&stdin)
        .arg(&to)
        .arg(&force)
        .arg(&dry_run)
        // Exactly one source: `--from` or `--stdin`.
        .group(
            Group::new("source")
                .member(&from)
                .member(&stdin)
                .required()
                .exclusive(),
        )
        // `--force` makes no sense without somewhere to write to.
        .requires(&force, &to)
        .exclusive([force.id(), dry_run.id()]);

    // `parse()` would print and exit here; `try_parse()` hands the error
    // back so the program decides what to do with it.
    let m = match cmd.try_parse() {
        Ok(m) => m,
        Err(e) if e.is_display() => {
            // `--help` and `--version` are not failures.
            e.print()?;
            return Ok(());
        }
        Err(e) => {
            // Usage errors can be inspected, decorated and exited by hand.
            e.print()?;
            match e.kind() {
                ErrorKind::UnknownOption | ErrorKind::UnknownSubcommand => {
                    eprintln!("(hint: run with --help to see what is accepted)");
                }
                ErrorKind::Conflict | ErrorKind::MissingRequired => {
                    eprintln!("(hint: check the constraints described in --help)");
                }
                _ => {}
            }
            std::process::exit(e.exit_code());
        }
    };

    // Checks the parser cannot express become `Error::custom`, so they are
    // rendered and exit like any other error (status 2).
    let source = m.get(&from);
    let to = m.get(&to);
    if let (Some(src), Some(dst)) = (&source, &to) {
        if src == dst {
            return Err(Error::custom(format!(
                "source and destination are the same file: {}",
                src.display()
            ))
            .with_tip("pass a different --to path"));
        }
    }

    let src = match source {
        Some(p) => p.display().to_string(),
        None => "<stdin>".to_owned(),
    };
    let dst = to.map_or_else(|| "<stdout>".to_owned(), |p| p.display().to_string());
    if m.get(&dry_run) {
        println!("would copy {src} -> {dst}");
    } else {
        println!(
            "copying {src} -> {dst}{}",
            if m.get(&force) { " (overwriting)" } else { "" }
        );
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        // Prints to the right stream, in colour when the `color` feature is
        // on and the stream is a terminal, then exits with 2.
        e.exit();
    }
}
