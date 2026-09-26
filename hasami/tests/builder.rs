//! Behavioural tests for the declarative layer (default features).
#![cfg(feature = "help")]

use std::ffi::OsString;
use std::path::PathBuf;

use hasami::{Arg, Command, ErrorKind, Group, Source, Subcommand};

fn greet() -> (Command, Arg<u32>, Arg<bool>, Arg<String>) {
    let number = Arg::new("number")
        .short('n')
        .value::<u32>()
        .default(1)
        .help("How many times");
    let shout = Arg::new("shout").help("Use upper case");
    let thing = Arg::positional::<String>("THING")
        .required()
        .help("Whom to greet");
    let cmd = Command::new("greet")
        .version("1.2.3")
        .about("Greet someone")
        .arg(&number)
        .arg(&shout)
        .arg(&thing);
    (cmd, number, shout, thing)
}

#[test]
fn basic_parse_and_typed_get() {
    let (cmd, number, shout, thing) = greet();
    let m = cmd.try_parse_args(["-n", "2", "--shout", "world"]).unwrap();
    assert_eq!(m.get(&number), 2);
    assert!(m.get(&shout));
    assert_eq!(m.get(&thing), "world");
    assert!(m.contains(&number));
    assert_eq!(m.source(&number), Some(Source::CommandLine));
    assert_eq!(m.occurrences(&number), 1);
    assert_eq!(m.raw(&number), vec![OsString::from("2").as_os_str()]);
}

#[test]
fn defaults_apply_and_are_not_present() {
    let (cmd, number, shout, _) = greet();
    let m = cmd.try_parse_args(["x"]).unwrap();
    assert_eq!(m.get(&number), 1);
    assert!(!m.get(&shout));
    assert!(!m.contains(&number));
    assert_eq!(m.source(&number), Some(Source::Default));
    assert_eq!(m.occurrences(&number), 0);
    assert_eq!(m.source(&shout), None);
}

#[test]
fn all_option_syntaxes() {
    let (cmd, number, _, _) = greet();
    for args in [
        &["-n", "3", "x"][..],
        &["-n3", "x"],
        &["-n=3", "x"],
        &["--number", "3", "x"],
        &["--number=3", "x"],
        &["x", "--number", "3"],
    ] {
        let m = cmd.try_parse_args(args.iter().copied()).unwrap();
        assert_eq!(m.get(&number), 3, "{args:?}");
    }
}

#[test]
fn repeated_single_value_is_an_error_by_default() {
    let (cmd, _, shout, _) = greet();
    let e = cmd.try_parse_args(["-n1", "-n2", "x"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::Repeated);
    assert_eq!(
        e.message(),
        "the argument '--number <NUMBER>' cannot be used multiple times"
    );
    let e = cmd.try_parse_args(["--shout", "--shout", "x"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::Repeated);
    assert_eq!(
        e.message(),
        "the argument '--shout' cannot be used multiple times"
    );
    let _ = shout;
}

#[test]
fn last_wins_opts_into_overriding() {
    let number = Arg::new("number").short('n').value::<u32>().last_wins();
    let cmd = Command::new("x").arg(&number);
    let m = cmd.try_parse_args(["-n1", "-n2"]).unwrap();
    assert_eq!(m.get(&number), Some(2));
    assert_eq!(m.occurrences(&number), 2);

    let number = Arg::new("number").short('n').value::<u32>();
    let shout = Arg::new("shout");
    let cmd = Command::new("x")
        .arg(&number)
        .arg(&shout)
        .args_override_self();
    let m = cmd
        .try_parse_args(["-n1", "--shout", "-n2", "--shout"])
        .unwrap();
    assert_eq!(m.get(&number), Some(2));
    assert!(m.get(&shout));
}

#[test]
fn try_parse_from_takes_bin_name() {
    let (cmd, _, _, thing) = greet();
    let m = cmd.try_parse_from(["/usr/bin/greet", "world"]).unwrap();
    assert_eq!(m.get(&thing), "world");
}

#[test]
fn parse_with_existing_core_parser() {
    let (cmd, _, _, thing) = greet();
    let mut p = hasami::lex::Parser::from_args(["skip", "world"]);
    assert!(matches!(
        p.next().unwrap(),
        Some(hasami::lex::Arg::Value(_))
    ));
    let m = cmd.try_parse_with(&mut p).unwrap();
    assert_eq!(m.get(&thing), "world");
}

// ------------------------------------------------------------- errors

fn err_text(cmd: &Command, args: &[&str]) -> String {
    cmd.try_parse_args(args.iter().copied())
        .unwrap_err()
        .to_string()
}

#[test]
fn unknown_option() {
    let (cmd, ..) = greet();
    let e = cmd.try_parse_args(["--nubmer", "2", "x"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownOption);
    assert_eq!(e.exit_code(), 2);
    let tip = if cfg!(feature = "suggest") {
        "a similar argument exists: '--number'"
    } else {
        "to pass '--nubmer' as a value, use '-- --nubmer'"
    };
    assert_eq!(
        e.to_string(),
        format!(
            "error: unexpected argument '--nubmer' found\n\n  tip: {tip}\n\nUsage: greet [OPTIONS] <THING>\n\nFor more information, try '--help'."
        )
    );
    let e = cmd.try_parse_args(["-z", "x"]).unwrap_err();
    assert_eq!(e.message(), "unexpected argument '-z' found");
}

#[test]
fn missing_required_positional() {
    let (cmd, ..) = greet();
    let e = cmd.try_parse_args(["-n", "2"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::MissingRequired);
    assert_eq!(
        e.to_string(),
        "error: the following required arguments were not provided:\n  <THING>\n\nUsage: greet [OPTIONS] <THING>\n\nFor more information, try '--help'."
    );
}

#[test]
fn missing_required_option() {
    let name = Arg::new("name").value::<String>().required();
    let cmd = Command::new("x").arg(&name);
    assert_eq!(
        err_text(&cmd, &[]),
        "error: the following required arguments were not provided:\n  --name <NAME>\n\nUsage: x [OPTIONS] --name <NAME>\n\nFor more information, try '--help'."
    );
}

#[test]
fn missing_value() {
    let (cmd, ..) = greet();
    let e = cmd.try_parse_args(["x", "--number"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::MissingValue);
    assert_eq!(
        e.message(),
        "a value is required for '--number <NUMBER>' but none was supplied"
    );
}

#[test]
fn invalid_value() {
    let (cmd, ..) = greet();
    let e = cmd.try_parse_args(["-n", "many", "x"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::InvalidValue);
    assert_eq!(
        e.message(),
        "invalid value 'many' for '--number <NUMBER>': invalid digit found in string"
    );
}

#[test]
fn flag_given_a_value() {
    let (cmd, ..) = greet();
    let e = cmd.try_parse_args(["--shout=yes", "x"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnexpectedValue);
    assert_eq!(
        e.message(),
        "unexpected value 'yes' for '--shout', which takes no value"
    );
}

#[test]
fn too_many_positionals() {
    let (cmd, ..) = greet();
    let e = cmd.try_parse_args(["a", "b"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnexpectedArgument);
    assert_eq!(e.message(), "unexpected argument 'b' found");
    assert_eq!(
        e.tip(),
        Some("'<THING>' was already given; the command accepts 1 positional argument")
    );

    let cmd = Command::new("none");
    let e = cmd.try_parse_args(["a"]).unwrap_err();
    assert_eq!(e.tip(), Some("the command takes no positional arguments"));
}

#[test]
fn double_dash_makes_options_positional() {
    let (cmd, _, shout, thing) = greet();
    let m = cmd.try_parse_args(["--", "--shout"]).unwrap();
    assert_eq!(m.get(&thing), "--shout");
    assert!(!m.get(&shout));
}

#[test]
fn custom_error_conversions() {
    let e: hasami::Error = "boom".into();
    assert_eq!(e.kind(), ErrorKind::Custom);
    assert_eq!(e.to_string(), "error: boom");
    let e = hasami::Error::custom("x").with_tip("y");
    assert_eq!(e.to_string(), "error: x\n\n  tip: y");
}

// ------------------------------------------------------- help/version

#[test]
fn help_flag_returns_display_help() {
    let (cmd, ..) = greet();
    for args in [&["--help"][..], &["-h"], &["x", "-h"], &["-hn"]] {
        let e = cmd.try_parse_args(args.iter().copied()).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::DisplayHelp, "{args:?}");
        assert_eq!(e.exit_code(), 0);
        assert!(e.is_display());
        assert_eq!(e.to_string(), cmd.render_help());
    }
}

#[test]
fn help_text_layout() {
    let (cmd, ..) = greet();
    assert_eq!(
        cmd.render_help(),
        "\
Greet someone

Usage: greet [OPTIONS] <THING>

Arguments:
  <THING>  Whom to greet

Options:
  -n, --number <NUMBER>  How many times [default: 1]
      --shout            Use upper case
  -h, --help             Print help
  -V, --version          Print version
"
    );
}

#[test]
fn version_flag() {
    let (cmd, ..) = greet();
    let e = cmd.try_parse_args(["-V"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::DisplayVersion);
    assert_eq!(e.to_string(), "greet 1.2.3");
    let e = cmd.try_parse_args(["--version"]).unwrap_err();
    assert_eq!(e.to_string(), "greet 1.2.3");
}

#[test]
fn no_version_flag_without_version() {
    let cmd = Command::new("x");
    let e = cmd.try_parse_args(["--version"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownOption);
    assert!(!cmd.render_help().contains("--version"));
}

#[test]
fn user_defined_h_wins_over_help() {
    let host = Arg::with_id("host").short('h').value::<String>();
    let cmd = Command::new("x").arg(&host);
    let m = cmd.try_parse_args(["-h", "example.org"]).unwrap();
    assert_eq!(m.get(&host).as_deref(), Some("example.org"));
    assert_eq!(
        cmd.try_parse_args(["--help"]).unwrap_err().kind(),
        ErrorKind::DisplayHelp
    );
}

#[test]
fn disable_help_and_version() {
    let cmd = Command::new("x")
        .version("1")
        .disable_help()
        .disable_version();
    assert_eq!(
        cmd.try_parse_args(["--help"]).unwrap_err().kind(),
        ErrorKind::UnknownOption
    );
    assert_eq!(
        cmd.try_parse_args(["-V"]).unwrap_err().kind(),
        ErrorKind::UnknownOption
    );
    assert_eq!(cmd.render_usage(), "x");
}

#[test]
fn help_layout_with_everything() {
    let verbose = Arg::new("verbose").short('v').count().help("More output");
    let out = Arg::new("output")
        .short('o')
        .value::<PathBuf>()
        .help("Where to write");
    let color = Arg::new("color")
        .value::<String>()
        .default_missing("always".to_owned())
        .possible(["auto", "always", "never"])
        .default("auto".to_owned())
        .help("When to colour");
    let files = Arg::positional::<PathBuf>("FILE").many().help("Inputs");
    let secret = Arg::new("secret").hidden();
    let very_long = Arg::new("a-very-long-option-name-indeed")
        .value::<String>()
        .value_name("SOMETHING")
        .help("Wraps to the next line");
    let cmd = Command::new("tool")
        .about("Short")
        .long_about("Long description\nwith two lines")
        .after_help("See also: nothing")
        .arg(&verbose)
        .arg(&out)
        .arg(&color)
        .arg(&files)
        .arg(&secret)
        .arg(&very_long)
        .subcommand(Command::new("build").about("Build it"))
        .subcommand(Subcommand::lazy("hidden", || Command::new("hidden")).hidden())
        .subcommand(Subcommand::lazy("run", || Command::new("run")).about("Run it\nsecond line"));
    assert_eq!(
        cmd.render_help(),
        "\
Long description
with two lines

Usage: tool [OPTIONS] [FILE]... [COMMAND]

Commands:
  build  Build it
  run    Run it
         second line

Arguments:
  [FILE]...  Inputs

Options:
  -v, --verbose          More output
  -o, --output <OUTPUT>  Where to write
      --color[=<COLOR>]  When to colour [default: auto] [possible values: auto, always, never]
      --a-very-long-option-name-indeed <SOMETHING>
                         Wraps to the next line
  -h, --help             Print help

See also: nothing
"
    );
}

// --------------------------------------------------------- value kinds

#[test]
fn count_flag() {
    let v = Arg::new("verbose").short('v').count();
    let cmd = Command::new("x").arg(&v);
    assert_eq!(cmd.try_parse_args(["-vvv"]).unwrap().get(&v), 3);
    assert_eq!(cmd.try_parse_args(["-v", "--verbose"]).unwrap().get(&v), 2);
    assert_eq!(cmd.try_parse_args([] as [&str; 0]).unwrap().get(&v), 0);
}

#[test]
fn optional_value_option() {
    let color = Arg::new("color")
        .short('c')
        .value::<String>()
        .default_missing("always".to_owned())
        .default("auto".to_owned());
    let file = Arg::positional::<String>("FILE");
    let cmd = Command::new("x").arg(&color).arg(&file);
    let m = cmd.try_parse_args([] as [&str; 0]).unwrap();
    assert_eq!(m.get(&color), "auto");
    let m = cmd.try_parse_args(["--color"]).unwrap();
    assert_eq!(m.get(&color), "always");
    assert!(m.contains(&color));
    let m = cmd.try_parse_args(["--color=never"]).unwrap();
    assert_eq!(m.get(&color), "never");
    let m = cmd.try_parse_args(["-cnever"]).unwrap();
    assert_eq!(m.get(&color), "never");
    // A following argument is never consumed as the value.
    let m = cmd.try_parse_args(["--color", "never"]).unwrap();
    assert_eq!(m.get(&color), "always");
    assert_eq!(m.get(&file).as_deref(), Some("never"));
}

#[test]
fn many_option_and_positional() {
    let inc = Arg::new("include").short('I').value::<String>().many();
    let files = Arg::positional::<String>("FILE").many().required();
    let cmd = Command::new("x").arg(&inc).arg(&files);
    let m = cmd
        .try_parse_args(["-Ia", "-I", "b", "f1", "--include=c", "f2", "f3"])
        .unwrap();
    assert_eq!(m.get(&inc), vec!["a", "b", "c"]);
    assert_eq!(m.get(&files), vec!["f1", "f2", "f3"]);
    let e = cmd.try_parse_args(["-Ia"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::MissingRequired);
    assert!(e.message().contains("<FILE>"));
}

#[test]
fn possible_values() {
    let mode = Arg::new("mode")
        .value::<String>()
        .possible(["fast", "slow"]);
    let cmd = Command::new("x").arg(&mode);
    assert_eq!(
        cmd.try_parse_args(["--mode", "fast"])
            .unwrap()
            .get(&mode)
            .as_deref(),
        Some("fast")
    );
    let e = cmd.try_parse_args(["--mode", "medium"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::InvalidValue);
    assert_eq!(
        e.message(),
        "invalid value 'medium' for '--mode <MODE>'\n  [possible values: fast, slow]"
    );
}

#[test]
fn custom_value_parsers() {
    let even = Arg::new("even").value_with(|s| {
        let n: u32 = s.parse().map_err(|e| format!("{e}"))?;
        if n % 2 == 0 {
            Ok(n)
        } else {
            Err(format!("{n} is odd"))
        }
    });
    let raw = Arg::new("raw").value_os();
    let cmd = Command::new("x").arg(&even).arg(&raw);
    assert_eq!(
        cmd.try_parse_args(["--even", "4"]).unwrap().get(&even),
        Some(4)
    );
    let e = cmd.try_parse_args(["--even", "3"]).unwrap_err();
    assert_eq!(
        e.message(),
        "invalid value '3' for '--even <EVEN>': 3 is odd"
    );
    let m = cmd.try_parse_args(["--raw", "anything"]).unwrap();
    assert_eq!(m.get(&raw), Some(OsString::from("anything")));
}

#[cfg(unix)]
#[test]
fn non_unicode_values() {
    use std::os::unix::ffi::OsStringExt;
    let b = |bytes: &[u8]| OsString::from_vec(bytes.to_vec());
    let raw = Arg::new("raw").value_os();
    let text = Arg::new("text").value::<String>();
    let path = Arg::positional_os_with("PATH", |s| {
        Ok::<_, std::convert::Infallible>(PathBuf::from(s))
    });
    let cmd = Command::new("x").arg(&raw).arg(&text).arg(&path);

    let m = cmd.try_parse_args([b(b"--raw=\xff"), b(b"\xfe")]).unwrap();
    assert_eq!(m.get(&raw), Some(b(b"\xff")));
    assert_eq!(m.get(&path), Some(PathBuf::from(b(b"\xfe"))));

    let e = cmd.try_parse_args([b(b"--text=\xff")]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::NonUnicode);
    assert_eq!(
        e.message(),
        "invalid value '\u{FFFD}' for '--text <TEXT>': not valid unicode"
    );

    let e = cmd.try_parse_args([b(b"-\xff")]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownOption);
    assert_eq!(e.tip(), Some("the option letter is not valid unicode"));
}

#[test]
fn aliases() {
    let colour = Arg::new("color")
        .alias("colour")
        .short('c')
        .short_alias('C');
    let cmd = Command::new("x").arg(&colour);
    for a in ["--color", "--colour", "-c", "-C"] {
        assert!(cmd.try_parse_args([a]).unwrap().get(&colour), "{a}");
    }
    assert!(!cmd.render_help().contains("colour"));
}

// --------------------------------------------------------- constraints

#[test]
fn exclusive_group() {
    let json = Arg::new("json");
    let yaml = Arg::new("yaml");
    let cmd = Command::new("x")
        .arg(&json)
        .arg(&yaml)
        .exclusive([json.id(), yaml.id()]);
    assert!(cmd.try_parse_args(["--json"]).is_ok());
    let e = cmd.try_parse_args(["--json", "--yaml"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::Conflict);
    assert_eq!(
        e.message(),
        "the argument '--json' cannot be used with '--yaml'"
    );
}

#[test]
fn required_group() {
    let json = Arg::new("json");
    let yaml = Arg::new("yaml");
    let cmd = Command::new("x").arg(&json).arg(&yaml).group(
        Group::new("format")
            .member(&json)
            .member(&yaml)
            .required()
            .exclusive(),
    );
    assert!(cmd.try_parse_args(["--yaml"]).is_ok());
    let e = cmd.try_parse_args([] as [&str; 0]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::MissingRequired);
    assert_eq!(
        e.message(),
        "one of the following arguments is required: --json, --yaml"
    );
    assert_eq!(
        cmd.try_parse_args(["--json", "--yaml"]).unwrap_err().kind(),
        ErrorKind::Conflict
    );
}

#[test]
fn default_does_not_count_as_present_for_groups() {
    let a = Arg::new("a").value::<u8>().default(1);
    let b = Arg::new("b");
    let cmd = Command::new("x")
        .arg(&a)
        .arg(&b)
        .exclusive([a.id(), b.id()]);
    assert!(cmd.try_parse_args(["--b"]).is_ok());
    assert!(cmd.try_parse_args(["--a", "2", "--b"]).is_err());
}

#[test]
fn requires() {
    let out = Arg::new("output").value::<String>();
    let force = Arg::new("force");
    let cmd = Command::new("x")
        .arg(&out)
        .arg(&force)
        .requires(&force, &out);
    assert!(cmd.try_parse_args(["--force", "--output", "f"]).is_ok());
    assert!(cmd.try_parse_args(["--output", "f"]).is_ok());
    let e = cmd.try_parse_args(["--force"]).unwrap_err();
    assert_eq!(
        e.message(),
        "the argument '--force' requires '--output <OUTPUT>', which was not provided"
    );
}

// --------------------------------------------------------- subcommands

fn repo() -> (Command, Arg<bool>, Arg<String>, Arg<bool>) {
    let verbose = Arg::new("verbose").short('v').global();
    let name = Arg::positional::<String>("NAME").required();
    let force = Arg::new("force").short('f');
    let add = Command::new("add")
        .about("Add a thing")
        .arg(&name)
        .arg(&force);
    let remove_name = name.clone();
    let cmd = Command::new("repo")
        .arg(&verbose)
        .subcommand(add)
        .subcommand_lazy("remove", "Remove a thing", move || {
            Command::new("remove").arg(&remove_name)
        })
        .subcommand(
            Subcommand::lazy("list", || Command::new("list"))
                .alias("ls")
                .about("List"),
        );
    (cmd, verbose, name, force)
}

#[test]
fn subcommand_dispatch() {
    let (cmd, verbose, name, force) = repo();
    let m = cmd.try_parse_args(["add", "thing", "-f"]).unwrap();
    let (sub, sm) = m.subcommand().unwrap();
    assert_eq!(sub, "add");
    assert_eq!(sm.get(&name), "thing");
    assert!(sm.get(&force));
    assert!(!m.get(&verbose));
    assert_eq!(m.subcommand_name(), Some("add"));
    assert!(m.subcommand_matches("add").is_some());
    assert!(m.subcommand_matches("remove").is_none());
}

#[test]
fn lazy_subcommand_and_alias() {
    let (cmd, _, name, _) = repo();
    let m = cmd.try_parse_args(["remove", "thing"]).unwrap();
    assert_eq!(m.subcommand().unwrap().1.get(&name), "thing");
    let m = cmd.try_parse_args(["ls"]).unwrap();
    assert_eq!(m.subcommand_name(), Some("list"));
}

#[test]
fn global_option_after_subcommand() {
    let (cmd, verbose, ..) = repo();
    let m = cmd.try_parse_args(["add", "thing", "-v"]).unwrap();
    assert!(m.get(&verbose));
    let m = cmd.try_parse_args(["-v", "add", "thing"]).unwrap();
    assert!(m.get(&verbose));
    // Non-global parent options are not visible in the subcommand.
    let (cmd, ..) = repo();
    let e = cmd.try_parse_args(["add", "thing", "--nope"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownOption);
    assert_eq!(e.usage(), Some("repo add [OPTIONS] <NAME>"));
}

#[test]
fn unknown_subcommand() {
    let (cmd, ..) = repo();
    let e = cmd.try_parse_args(["push"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownSubcommand);
    assert_eq!(e.message(), "unrecognized subcommand 'push'");
    assert_eq!(
        e.tip(),
        Some("see 'repo --help' for the list of subcommands")
    );
    assert_eq!(e.usage(), Some("repo [OPTIONS] [COMMAND]"));
}

#[test]
fn subcommand_required() {
    let (cmd, ..) = repo();
    let cmd = cmd.subcommand_required();
    let e = cmd.try_parse_args(["-v"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::MissingSubcommand);
    assert_eq!(e.tip(), Some("available subcommands: add, remove, list"));
    assert_eq!(e.usage(), Some("repo [OPTIONS] <COMMAND>"));
    assert!(cmd.try_parse_args(["list"]).is_ok());
}

#[test]
fn subcommand_help_uses_path() {
    let (cmd, ..) = repo();
    let e = cmd.try_parse_args(["add", "--help"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::DisplayHelp);
    assert_eq!(
        e.to_string(),
        "\
Add a thing

Usage: repo add [OPTIONS] <NAME>

Arguments:
  <NAME>

Options:
  -f, --force
  -h, --help   Print help
"
    );
}

#[test]
fn missing_required_in_subcommand_uses_sub_usage() {
    let (cmd, ..) = repo();
    let e = cmd.try_parse_args(["add"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::MissingRequired);
    assert_eq!(e.usage(), Some("repo add [OPTIONS] <NAME>"));
}

#[test]
fn nested_subcommands() {
    let deep = Arg::new("deep");
    let inner = Command::new("inner").arg(&deep);
    let mid = Command::new("mid").subcommand(inner);
    let cmd = Command::new("top").subcommand(mid);
    let m = cmd.try_parse_args(["mid", "inner", "--deep"]).unwrap();
    let (n1, m1) = m.subcommand().unwrap();
    let (n2, m2) = m1.subcommand().unwrap();
    assert_eq!((n1, n2), ("mid", "inner"));
    assert!(m2.get(&deep));
    let e = cmd.try_parse_args(["mid", "inner", "-h"]).unwrap_err();
    assert!(e.to_string().starts_with("Usage: top mid inner [OPTIONS]"));
}

#[test]
fn positional_and_subcommand_coexist() {
    let file = Arg::positional::<String>("FILE");
    let cmd = Command::new("x").arg(&file).subcommand(Command::new("sub"));
    let m = cmd.try_parse_args(["sub"]).unwrap();
    assert_eq!(m.subcommand_name(), Some("sub"));
    let m = cmd.try_parse_args(["other"]).unwrap();
    assert_eq!(m.get(&file).as_deref(), Some("other"));
    assert_eq!(m.subcommand_name(), None);
}

// -------------------------------------------------------- introspection

#[test]
fn introspection_getters() {
    let (cmd, ..) = greet();
    assert_eq!(cmd.name(), "greet");
    assert_eq!(cmd.get_version(), Some("1.2.3"));
    assert_eq!(cmd.get_about(), Some("Greet someone"));
    let defs = cmd.args();
    assert_eq!(defs.len(), 3);
    let n = cmd.find_arg("number").unwrap();
    assert_eq!(n.long(), Some("number"));
    assert_eq!(n.short(), Some('n'));
    assert!(n.takes_value());
    assert_eq!(n.value_name(), Some("NUMBER"));
    assert_eq!(n.default_text(), Some("1"));
    assert_eq!(n.display_name(), "--number <NUMBER>");
    assert_eq!(n.spellings(), vec!["-n", "--number"]);
    let t = cmd.find_arg("THING").unwrap();
    assert!(t.is_positional());
    assert!(t.is_required());
    assert_eq!(t.display_name(), "<THING>");
    assert!(cmd.has_help_flag());
    assert!(cmd.has_version_flag());
    let (repo, ..) = repo();
    let sub = repo.find_subcommand("ls").unwrap();
    assert_eq!(sub.name(), "list");
    assert_eq!(sub.build().name(), "list");
}

#[test]
fn try_get_with_foreign_key_is_none() {
    let (cmd, ..) = greet();
    let other = Arg::new("other");
    let m = cmd.try_parse_args(["x"]).unwrap();
    assert_eq!(m.try_get(&other), None);
}

#[test]
fn debug_output_is_readable() {
    let (cmd, ..) = greet();
    let m = cmd.try_parse_args(["-n", "2", "x"]).unwrap();
    let dbg = format!("{m:?}");
    assert!(dbg.contains("number: [\"2\"]"));
    assert!(dbg.contains("THING: [\"x\"]"));
}
