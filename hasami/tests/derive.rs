//! `#[derive(Args)]` / `#[derive(Commands)]`: the same definition through
//! the derive and through `cli!` must produce identical commands, matches,
//! help and errors.
#![cfg(all(feature = "derive", feature = "help"))]

use std::path::PathBuf;

use hasami::{Args, Cli, Commands, ErrorKind, Subcommands};

/// Greet someone
#[derive(Args, Debug, Clone, PartialEq)]
#[hasami(name = "greet", version = "1.0")]
struct DGreet {
    /// Name of the person
    #[hasami(long, short = 'n')]
    name: String,
    /// Number of times
    #[hasami(long, default = 1)]
    count: u8,
    /// Use upper case
    shout: bool,
    #[hasami(subcommand)]
    cmd: Option<DCmd>,
}

#[derive(Commands, Debug, Clone, PartialEq)]
enum DCmd {
    /// Add a thing
    Add(DAdd),
    /// Remove a thing
    #[hasami(name = "rm", alias = "remove")]
    Remove(DRemove),
    /// List things
    List,
}

#[derive(Args, Debug, Clone, PartialEq)]
struct DAdd {
    /// What to add
    #[hasami(positional)]
    thing: String,
    /// Be forceful
    #[hasami(short)]
    force: bool,
}

/// Remove things
#[derive(Args, Debug, Clone, PartialEq)]
struct DRemove {
    /// What to remove
    #[hasami(positional)]
    things: Vec<String>,
}

hasami::cli! {
    /// Greet someone
    #[derive(Debug, Clone, PartialEq)]
    #[name = "greet", version = "1.0"]
    struct MGreet {
        /// Name of the person
        #[long, short = 'n'] name: String,
        /// Number of times
        #[long, default = 1] count: u8,
        /// Use upper case
        shout: bool,
        #[subcommand] cmd: Option<MCmd>,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum MCmd {
        /// Add a thing
        Add(MAdd),
        /// Remove a thing
        #[name = "rm", alias = "remove"]
        Remove(MRemove),
        /// List things
        List,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct MAdd {
        /// What to add
        #[positional] thing: String,
        /// Be forceful
        #[short] force: bool,
    }

    /// Remove things
    #[derive(Debug, Clone, PartialEq)]
    struct MRemove {
        /// What to remove
        #[positional] things: Vec<String>,
    }
}

const INPUTS: &[&[&str]] = &[
    &[],
    &["-n", "bob"],
    &["--name=bob", "--count", "3", "--shout", "add", "cake", "-f"],
    &["-nbob", "rm", "a", "b"],
    &["-nbob", "remove"],
    &["-nbob", "list"],
    &["-n", "x", "--count", "300"],
    &["-n", "x", "push"],
    &["--help"],
    &["-nx", "add", "--help"],
    &["-nx", "rm", "-h"],
    &["-V"],
    &["-n", "x", "add"],
    &["--nope"],
    &["-n", "x", "--shout=1"],
];

#[test]
fn derive_and_macro_are_equivalent() {
    let d = DGreet::command();
    let m = MGreet::command();
    assert_eq!(d.render_help(), m.render_help());
    assert_eq!(d.render_usage(), m.render_usage());
    for sub in ["add", "rm", "list"] {
        let a = d.find_subcommand(sub).unwrap().build();
        let b = m.find_subcommand(sub).unwrap().build();
        assert_eq!(a.render_help(), b.render_help(), "{sub}");
    }
    for args in INPUTS {
        let a = d.try_parse_args(args.iter().copied());
        let b = m.try_parse_args(args.iter().copied());
        match (a, b) {
            (Ok(a), Ok(b)) => assert_eq!(format!("{a:?}"), format!("{b:?}"), "{args:?}"),
            (Err(a), Err(b)) => {
                assert_eq!(a.kind(), b.kind(), "{args:?}");
                assert_eq!(a.to_string(), b.to_string(), "{args:?}");
            }
            (a, b) => panic!("{args:?}: derive={a:?} macro={b:?}"),
        }
    }
    for args in INPUTS {
        let a = DGreet::try_parse_args(args.iter().copied()).map(|a| format!("{a:?}"));
        let b = MGreet::try_parse_args(args.iter().copied()).map(|b| format!("{b:?}"));
        // Type names differ (DGreet vs MGreet); compare after normalising.
        let norm = |s: String| {
            s.replace("DGreet", "X")
                .replace("MGreet", "X")
                .replace("DCmd", "C")
                .replace("MCmd", "C")
                .replace("DAdd", "A")
                .replace("MAdd", "A")
                .replace("DRemove", "R")
                .replace("MRemove", "R")
        };
        match (a, b) {
            (Ok(a), Ok(b)) => assert_eq!(norm(a), norm(b), "{args:?}"),
            (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string(), "{args:?}"),
            (a, b) => panic!("{args:?}: derive={a:?} macro={b:?}"),
        }
    }
}

#[test]
fn derive_parses_typed_values() {
    let g = DGreet::try_parse_args(["-n", "bob", "add", "cake", "-f"]).unwrap();
    assert_eq!(
        g,
        DGreet {
            name: "bob".into(),
            count: 1,
            shout: false,
            cmd: Some(DCmd::Add(DAdd {
                thing: "cake".into(),
                force: true
            })),
        }
    );
    assert_eq!(DGreet::ABOUT, Some(" Greet someone"));
    assert_eq!(DAdd::ABOUT, None);
    assert_eq!(DCmd::subcommands().len(), 3);
    let e = DGreet::try_parse_args(["-n", "x", "zap"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownSubcommand);
}

/// Every field kind through the derive.
#[derive(Args, Debug)]
#[hasami(name = "kinds", about = "Kinds of fields", after_help = "Bye.")]
struct Kinds {
    /// Verbosity
    #[hasami(short, count)]
    verbose: usize,
    /// Plain usize
    size: usize,
    /// Output
    #[hasami(short, value_name = "PATH")]
    output: Option<PathBuf>,
    /// Includes
    #[hasami(short = 'I', long = "include", alias = "inc")]
    includes: Vec<String>,
    /// Colour
    #[hasami(default = String::from("auto"), default_missing = String::from("always"), possible = ["auto", "always", "never"])]
    color: String,
    /// Hidden
    #[hasami(hidden, help = "not shown")]
    secret: bool,
    #[hasami(global)]
    verbose_global: bool,
    /// First
    #[hasami(positional)]
    first: String,
    /// Rest
    #[hasami(positional)]
    rest: Vec<PathBuf>,
    #[hasami(env = "KINDS_LEVEL")]
    level: Option<u8>,
}

#[test]
fn all_field_kinds_through_derive() {
    let k = Kinds::try_parse_args([
        "-vv", "--size", "3", "-o", "out", "-Ia", "--inc", "b", "--color", "f", "g", "h",
    ])
    .unwrap();
    assert_eq!(k.verbose, 2);
    assert_eq!(k.size, 3);
    assert_eq!(k.output, Some(PathBuf::from("out")));
    assert_eq!(k.includes, vec!["a", "b"]);
    assert_eq!(k.color, "always");
    assert!(!k.secret);
    assert_eq!(k.first, "f");
    assert_eq!(k.rest, vec![PathBuf::from("g"), PathBuf::from("h")]);
    assert_eq!(k.level, None);
    assert!(!k.verbose_global);
    let help = Kinds::command().render_help();
    assert!(help.starts_with(
        "Kinds of fields\n\nUsage: kinds [OPTIONS] --size <SIZE> <FIRST> [REST]...\n"
    ));
    assert!(help.contains("  -I, --include <INCLUDES>...  Includes\n"));
    assert!(help.ends_with("\nBye.\n"));
}

#[test]
fn required_subcommand_through_derive() {
    #[derive(Args)]
    struct Top {
        #[hasami(subcommand)]
        cmd: Only,
    }
    #[derive(Commands)]
    enum Only {
        Remove(DRemove),
    }
    let e = Top::try_parse_args([] as [&str; 0])
        .err()
        .expect("missing subcommand");
    assert_eq!(e.kind(), ErrorKind::MissingSubcommand);
    let Ok(t) = Top::try_parse_args(["remove", "x"]) else {
        panic!("parses")
    };
    assert!(matches!(t.cmd, Only::Remove(DRemove { things }) if things == ["x"]));
    assert!(
        Top::command()
            .render_help()
            .contains("remove  Remove things")
    );
}
