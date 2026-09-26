//! The `cli!` macro: the SPEC example, every field kind, and equivalence
//! with the hand-built `Command`.
#![cfg(feature = "help")]

use std::path::PathBuf;

use hasami::{Arg, Cli, Command, ErrorKind};

hasami::cli! {
    /// Greet someone
    #[derive(Debug, Clone, PartialEq)]
    #[name = "greet", version = "1.0"]
    struct Args {
        /// Name of the person
        #[long, short = 'n'] name: String,
        /// Number of times
        #[long, default = 1] count: u8,
        /// Use upper case
        shout: bool,
        #[subcommand] cmd: Option<Cmd>,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum Cmd {
        /// Add a thing
        Add(AddArgs),
        /// Remove a thing
        #[name = "rm", alias = "remove"]
        Remove(RemoveArgs),
        /// List things
        List,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct AddArgs {
        /// What to add
        #[positional] thing: String,
        /// Be forceful
        #[short] force: bool,
    }

    /// Remove things
    #[derive(Debug, Clone, PartialEq)]
    struct RemoveArgs {
        /// What to remove
        #[positional] things: Vec<String>,
    }
}

#[test]
fn spec_example_parses() {
    let a = Args::try_parse_args(["-n", "bob"]).unwrap();
    assert_eq!(
        a,
        Args {
            name: "bob".into(),
            count: 1,
            shout: false,
            cmd: None
        }
    );
    let a = Args::try_parse_args(["--name=bob", "--count", "3", "--shout", "add", "cake", "-f"])
        .unwrap();
    assert_eq!(a.count, 3);
    assert!(a.shout);
    assert_eq!(
        a.cmd,
        Some(Cmd::Add(AddArgs {
            thing: "cake".into(),
            force: true
        }))
    );
    let a = Args::try_parse_args(["-nbob", "rm", "a", "b"]).unwrap();
    assert_eq!(
        a.cmd,
        Some(Cmd::Remove(RemoveArgs {
            things: vec!["a".into(), "b".into()]
        }))
    );
    let a = Args::try_parse_args(["-nbob", "remove"]).unwrap();
    assert_eq!(a.cmd, Some(Cmd::Remove(RemoveArgs { things: vec![] })));
    let a = Args::try_parse_args(["-nbob", "list"]).unwrap();
    assert_eq!(a.cmd, Some(Cmd::List));
}

#[test]
fn spec_example_errors() {
    let e = Args::try_parse_args([] as [&str; 0]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::MissingRequired);
    assert!(e.message().contains("--name <NAME>"));
    let e = Args::try_parse_args(["-n", "x", "--count", "300"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::InvalidValue);
    assert_eq!(
        e.message(),
        "invalid value '300' for '--count <COUNT>': number too large to fit in target type"
    );
    let e = Args::try_parse_args(["-n", "x", "push"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownSubcommand);
}

#[test]
fn spec_example_help() {
    let e = Args::try_parse_args(["--help"]).unwrap_err();
    assert_eq!(
        e.to_string(),
        "\
Greet someone

Usage: greet [OPTIONS] --name <NAME> [COMMAND]

Commands:
  add   Add a thing
  rm    Remove a thing
  list  List things

Options:
  -n, --name <NAME>    Name of the person
      --count <COUNT>  Number of times [default: 1]
      --shout          Use upper case
  -h, --help           Print help
  -V, --version        Print version
"
    );
    let e = Args::try_parse_args(["-nx", "add", "--help"]).unwrap_err();
    assert_eq!(
        e.to_string(),
        "\
Add a thing

Usage: greet add [OPTIONS] <THING>

Arguments:
  <THING>  What to add

Options:
  -f, --force  Be forceful
  -h, --help   Print help
"
    );
    assert_eq!(
        Args::try_parse_args(["-V"]).unwrap_err().to_string(),
        "greet 1.0"
    );
    assert_eq!(Args::ABOUT, Some(" Greet someone"));
    assert_eq!(AddArgs::ABOUT, None);
}

#[test]
fn subcommand_summary_falls_back_to_payload_doc() {
    hasami::cli! {
        struct Top {
            #[subcommand] cmd: Only,
        }
        enum Only {
            Remove(RemoveArgs),
        }
    }
    let help = Top::command().render_help();
    assert!(help.contains("remove  Remove things"), "{help}");
    let e = Top::try_parse_args([] as [&str; 0])
        .err()
        .expect("missing subcommand");
    assert_eq!(e.kind(), ErrorKind::MissingSubcommand);
    let Ok(t) = Top::try_parse_args(["remove", "x"]) else {
        panic!("parses")
    };
    assert!(matches!(t.cmd, Only::Remove(RemoveArgs { things }) if things == ["x"]));
}

hasami::cli! {
    /// Every field kind
    #[derive(Debug)]
    #[name = "kinds", about = "Kinds of fields", after_help = "Bye."]
    struct Kinds {
        /// Verbosity
        #[short, count] verbose: usize,
        /// Plain usize
        size: usize,
        /// Output
        #[short, value_name = "PATH"] output: Option<PathBuf>,
        /// Includes
        #[short = 'I', long = "include", alias = "inc"] includes: Vec<String>,
        /// Colour
        #[default = String::from("auto"), default_missing = String::from("always"), possible = ["auto", "always", "never"]]
        color: String,
        /// Hidden
        #[hidden, help = "not shown"] secret: bool,
        #[global] verbose_global: bool,
        /// First
        #[positional] first: String,
        /// Rest
        #[positional] rest: Vec<PathBuf>,
        #[env = "KINDS_LEVEL"] level: Option<u8>,
    }
}

#[test]
fn all_field_kinds() {
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

    let k = Kinds::try_parse_args(["--size=1", "--color=never", "x"]).unwrap();
    assert_eq!(k.color, "never");
    assert_eq!(k.verbose, 0);
    let e = Kinds::try_parse_args(["--size=1", "--color=blue", "x"]).unwrap_err();
    assert!(
        e.message()
            .contains("[possible values: auto, always, never]")
    );
}

#[test]
fn all_field_kinds_help() {
    assert_eq!(
        Kinds::command().render_help(),
        "\
Kinds of fields

Usage: kinds [OPTIONS] --size <SIZE> <FIRST> [REST]...

Arguments:
  <FIRST>    First
  [REST]...  Rest

Options:
  -v, --verbose                Verbosity
      --size <SIZE>            Plain usize
  -o, --output <PATH>          Output
  -I, --include <INCLUDES>...  Includes
      --color[=<COLOR>]        Colour [default: auto] [possible values: auto, always, never]
      --verbose-global
      --level <LEVEL>{env}
  -h, --help                   Print help

Bye.
"
        .replace(
            "{env}",
            if cfg!(feature = "env") {
                "          [env: KINDS_LEVEL]"
            } else {
                ""
            }
        )
    );
    let cmd = Kinds::command();
    assert_eq!(cmd.name(), "kinds");
    assert_eq!(cmd.find_arg("secret").unwrap().help(), Some("not shown"));
    assert!(cmd.find_arg("verbose_global").unwrap().is_global());
    assert_eq!(
        cmd.find_arg("includes").unwrap().aliases(),
        &["inc".to_string()]
    );
}

/// The macro must produce exactly what the builder produces.
#[test]
fn macro_and_builder_are_equivalent() {
    let name = Arg::new("name")
        .short('n')
        .value::<String>()
        .required()
        .help("Name of the person");
    let count = Arg::new("count")
        .value::<u8>()
        .default(1)
        .help("Number of times");
    let shout = Arg::new("shout").help("Use upper case");
    let thing = Arg::positional::<String>("thing")
        .value_name("THING")
        .required()
        .help("What to add");
    let force = Arg::new("force").short('f').help("Be forceful");
    let things = Arg::positional::<String>("things")
        .value_name("THINGS")
        .many()
        .help("What to remove");
    let by_hand = Command::new("greet")
        .version("1.0")
        .about("Greet someone")
        .arg(&name)
        .arg(&count)
        .arg(&shout)
        .subcommand(
            Command::new("add")
                .about("Add a thing")
                .arg(&thing)
                .arg(&force),
        )
        .subcommand(
            hasami::Subcommand::lazy("rm", move || {
                Command::new("rm").about("Remove things").arg(&things)
            })
            .about("Remove a thing")
            .alias("remove"),
        )
        .subcommand(hasami::Subcommand::lazy("list", || Command::new("list")).about("List things"));
    let by_macro = Args::command();

    assert_eq!(by_hand.render_help(), by_macro.render_help());
    for sub in ["add", "rm", "list"] {
        let a = by_hand.find_subcommand(sub).unwrap().build();
        let b = by_macro.find_subcommand(sub).unwrap().build();
        assert_eq!(a.render_help(), b.render_help(), "{sub}");
        assert_eq!(a.args().len(), b.args().len());
    }
    for args in [
        &["-n", "x"][..],
        &["-n", "x", "add", "y"],
        &["--count=2", "-nx", "--shout"],
        &["-nx", "remove", "a", "b"],
    ] {
        let a = by_hand.try_parse_args(args.iter().copied()).unwrap();
        let b = by_macro.try_parse_args(args.iter().copied()).unwrap();
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "{args:?}");
    }
    for args in [
        &[][..],
        &["-n", "x", "zap"],
        &["-n", "x", "--count", "300"],
        &["-n", "x", "add"],
    ] {
        let a = by_hand.try_parse_args(args.iter().copied()).unwrap_err();
        let b = by_macro.try_parse_args(args.iter().copied()).unwrap_err();
        assert_eq!(a.to_string(), b.to_string(), "{args:?}");
    }
}

#[test]
fn kebab_case_names() {
    hasami::cli! {
        struct K {
            #[subcommand] cmd: Option<KCmd>,
            dry_run: bool,
        }
        enum KCmd { DoThing, }
    }
    let k = K::try_parse_args(["--dry-run", "do-thing"]).unwrap();
    assert!(k.dry_run);
    assert!(matches!(k.cmd, Some(KCmd::DoThing)));
    assert_eq!(
        K::command().find_arg("dry_run").unwrap().long(),
        Some("dry-run")
    );
}

#[test]
fn from_matches_and_parse_from() {
    let m = Args::command()
        .try_parse_from(["greet", "-n", "z"])
        .unwrap();
    let a = Args::from_matches(&m).unwrap();
    assert_eq!(a.name, "z");
    let a = Args::try_parse_from(["greet", "-n", "z"]).unwrap();
    assert_eq!(a.name, "z");
    assert!(Args::try_parse_from(["greet"]).is_err());
}

// ------------------------------------------------- settings added for clap parity

hasami::value_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Level { Low = "low", High = "high" }
}

hasami::cli! {
    /// Parity settings
    #[derive(Debug)]
    #[name = "parity", version = "1", long_version = "1 (full)", before_help = "Banner"]
    #[infer_long_args, infer_subcommands, term_width = 80]
    struct Parity {
        /// Level
        #[value_enum, default = Level::Low, help_heading = "Tuning"] level: Level,
        /// Run
        #[short, greedy] exec: Vec<String>,
        /// Features
        #[short, delimiter = ','] features: Vec<String>,
        /// Force
        #[requires = "output", conflicts_with = "dry_run", visible_alias = "yes"] force: bool,
        /// Output
        #[long_help = "Where the result goes, at length"] output: Option<String>,
        /// Dry run
        dry_run: bool,
        /// Key
        #[required_if_eq = ["level", "high"]] key: Option<String>,
        /// Repeatable
        #[last_wins] name: Option<String>,
        #[subcommand] cmd: Option<ParityCmd>,
    }

    #[derive(Debug)]
    enum ParityCmd {
        /// Install
        #[visible_alias = "i"]
        Install,
        /// Run everything after it
        Run(RunArgs),
    }

    #[derive(Debug)]
    struct RunArgs {
        /// Program and arguments
        #[positional, trailing] args: Vec<String>,
    }
}

#[test]
fn parity_settings_through_the_macro() {
    let p = Parity::try_parse_args([
        "--level", "high", "--key", "k", "-e", "echo", "a", "-f", "x,y", "--name", "n1", "--name",
        "n2", "--forc", "--output", "o",
    ])
    .unwrap();
    assert_eq!(p.level, Level::High);
    assert_eq!(p.key.as_deref(), Some("k"));
    assert_eq!(p.exec, ["echo", "a"]);
    assert_eq!(p.features, ["x", "y"]);
    assert_eq!(p.name.as_deref(), Some("n2"));
    assert!(p.force);

    let e = Parity::try_parse_args(["--level", "high"]).unwrap_err();
    assert!(e.message().contains("--key <KEY>"), "{}", e.message());
    let e = Parity::try_parse_args(["--yes"]).unwrap_err();
    assert!(
        e.message().contains("requires '--output <OUTPUT>'"),
        "{}",
        e.message()
    );
    let e = Parity::try_parse_args(["--force", "--output", "o", "--dry-run"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::Conflict);
    let e = Parity::try_parse_args(["--output", "a", "--output", "b"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::Repeated);

    let p = Parity::try_parse_args(["i"]).unwrap();
    assert!(matches!(p.cmd, Some(ParityCmd::Install)));
    let p = Parity::try_parse_args(["run", "prog", "-x", "--y"]).unwrap();
    match p.cmd {
        Some(ParityCmd::Run(r)) => assert_eq!(r.args, ["prog", "-x", "--y"]),
        other => panic!("{other:?}"),
    }

    let cmd = Parity::command();
    let short = cmd.render_short_help();
    let long = cmd.render_help();
    assert!(long.starts_with("Banner\n\nParity settings\n\nUsage: parity"));
    assert!(
        short.contains("Output\n") && !short.contains("at length"),
        "{short}"
    );
    assert!(long.contains("Where the result goes, at length"), "{long}");
    assert!(long.contains("Tuning:\n"), "{long}");
    assert!(long.contains("[possible values: low, high]"), "{long}");
    assert!(long.contains("[aliases: yes]"), "{long}");
    assert!(long.contains("install  Install [aliases: i]"), "{long}");
    assert!(long.lines().all(|l| l.chars().count() <= 80), "{long}");
    assert_eq!(cmd.get_long_version(), Some("1 (full)"));
    let e = Parity::try_parse_args(["--version"]).unwrap_err();
    assert_eq!(e.message(), "parity 1 (full)");
}

#[test]
fn macro_definitions_validate() {
    assert!(Parity::command().validate().is_ok());
    assert!(Args::command().validate().is_ok());
}

hasami::cli! {
    /// Shared options
    #[derive(Debug)]
    struct Common {
        /// Say more
        #[short, global] verbose: bool,
        /// Config file
        #[short, env = "FLAT_CONFIG"] config: Option<String>,
    }

    /// Uses the shared options
    #[derive(Debug)]
    #[name = "flat"]
    struct Flat {
        #[flatten] common: Common,
        /// Own flag
        own: bool,
    }
}

#[test]
fn flatten_inlines_another_struct() {
    let f = Flat::try_parse_args(["-v", "-c", "x.toml", "--own"]).unwrap();
    assert!(f.common.verbose);
    assert_eq!(f.common.config.as_deref(), Some("x.toml"));
    assert!(f.own);
    let help = Flat::command().render_help();
    assert!(
        help.contains("-c, --config <CONFIG>  Config file"),
        "{help}"
    );
    assert!(help.contains("--own"), "{help}");
    assert!(Flat::command().validate().is_ok());
}

hasami::cli! {
    /// With examples
    #[name = "ex", example = "ex --n 1", example = "ex --n x"]
    struct Ex {
        /// N
        n: Option<u32>,
    }
}

#[test]
fn example_setting_through_the_macro() {
    let cmd = Ex::command();
    assert_eq!(cmd.get_examples(), ["ex --n 1", "ex --n x"]);
    assert_eq!(cmd.check_examples().unwrap_err().len(), 1);
    assert!(
        cmd.render_help()
            .contains("Examples:\n  ex --n 1\n  ex --n x\n")
    );
}
