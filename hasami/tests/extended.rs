//! The features added to close the gaps against clap: repeat handling,
//! definition validation, multi-value occurrences, delimiters, conditional
//! constraints, help customisation, `help` subcommand, prefix inference,
//! external subcommands, trailing positionals and value enums.

use std::ffi::OsString;

use hasami::{Arg, Command, ErrorKind, Group, Subcommand, ValueEnum};

// ------------------------------------------------------------ repeats

#[test]
fn repeated_global_flag_across_levels_is_an_error() {
    let verbose = Arg::new("verbose").short('v').global();
    let cmd = Command::new("app")
        .arg(&verbose)
        .subcommand(Command::new("sub"));
    let e = cmd.try_parse_args(["-v", "sub", "-v"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::Repeated);
    let counted = Arg::new("verbose").short('v').global().count();
    let cmd = Command::new("app")
        .arg(&counted)
        .subcommand(Command::new("sub"));
    let m = cmd.try_parse_args(["-v", "sub", "-v"]).unwrap();
    assert_eq!(m.get(&counted), 2);
}

// ---------------------------------------------------------- validation

fn problems(cmd: &Command) -> Vec<String> {
    cmd.validate().unwrap_err()
}

#[test]
fn validate_reports_every_problem() {
    let a = Arg::new("a").short('x');
    let b = Arg::with_id("b").long("a").short('x');
    let files = Arg::positional::<String>("FILES").many();
    let after = Arg::positional::<String>("AFTER");
    let mode = Arg::new("mode")
        .value::<String>()
        .possible(["fast", "slow"])
        .default("medium".to_owned());
    let cmd = Command::new("app")
        .arg(&a)
        .arg(&b)
        .arg(&files)
        .arg(&after)
        .arg(&mode)
        .requires(&a, &after)
        .group(Group::new("g").member_id("nope").exclusive())
        .subcommand(Command::new("sub"))
        .subcommand(Subcommand::from(Command::new("other")).alias("sub"));
    let p = problems(&cmd);
    let has = |text: &str| p.iter().any(|m| m.contains(text));
    assert!(has("long name '--a' is used more than once"), "{p:?}");
    assert!(has("short name '-x' is used more than once"), "{p:?}");
    assert!(
        has("positional 'AFTER' follows a repeated positional"),
        "{p:?}"
    );
    assert!(
        has("default 'medium' of 'mode' is not a possible value"),
        "{p:?}"
    );
    assert!(has("group 'g' refers to unknown argument 'nope'"), "{p:?}");
    assert!(
        has("exclusive group 'g' has fewer than two members"),
        "{p:?}"
    );
    assert!(has("subcommand name 'sub' is used more than once"), "{p:?}");
    assert!(p.iter().all(|m| m.starts_with("app: ")), "{p:?}");
}

#[test]
fn validate_checks_ordering_and_references() {
    let opt = Arg::positional::<String>("OPT");
    let req = Arg::positional::<String>("REQ").required();
    let cmd = Command::new("app").arg(&opt).arg(&req);
    assert!(
        problems(&cmd)
            .iter()
            .any(|m| m.contains("required positional 'REQ' follows an optional one"))
    );

    let dup = Arg::new("dup");
    let cmd = Command::new("app").arg(&dup).arg(&dup);
    assert!(
        problems(&cmd)
            .iter()
            .any(|m| m.contains("duplicate argument id 'dup'"))
    );

    let other = Arg::new("other");
    let a = Arg::new("a").requires(&other);
    let cmd = Command::new("app").arg(&a);
    assert!(
        problems(&cmd)
            .iter()
            .any(|m| m.contains("'a' refers to unknown argument 'other'"))
    );

    let cmd = Command::new("app").subcommand_required();
    assert!(
        problems(&cmd)
            .iter()
            .any(|m| m.contains("subcommand_required is set but there are no subcommands"))
    );
}

#[test]
fn validate_descends_into_lazy_subcommands() {
    let cmd = Command::new("app").subcommand(Subcommand::lazy("sub", || {
        let x = Arg::new("x");
        Command::new("sub").arg(&x).arg(&x)
    }));
    let p = problems(&cmd);
    assert_eq!(p, ["app sub: duplicate argument id 'x'"]);
}

#[test]
fn a_valid_definition_passes() {
    let name = Arg::new("name").short('n').value::<String>();
    let files = Arg::positional::<String>("FILE").many();
    let cmd = Command::new("app")
        .version("1")
        .arg(&name)
        .arg(&files)
        .subcommand(Command::new("sub"));
    assert!(cmd.validate().is_ok());
}

#[test]
#[should_panic(expected = "invalid command definition")]
fn parsing_an_invalid_definition_panics_in_debug_builds() {
    let dup = Arg::new("dup");
    let cmd = Command::new("app").arg(&dup).arg(&dup);
    let _ = cmd.try_parse_args(["--dup"]);
}

// ----------------------------------------------------- values per occurrence

#[test]
fn greedy_takes_every_following_value() {
    let exec = Arg::new("exec")
        .short('e')
        .value::<String>()
        .many()
        .greedy();
    let verbose = Arg::new("verbose").short('v');
    let cmd = Command::new("app").arg(&exec).arg(&verbose);
    let m = cmd.try_parse_args(["-e", "echo", "a", "b", "-v"]).unwrap();
    assert_eq!(m.get(&exec), ["echo", "a", "b"]);
    assert!(m.get(&verbose));
    // An attached value limits the occurrence to that one value.
    let m = cmd
        .try_parse_args(["--exec=echo", "-e", "ls", "x"])
        .unwrap();
    assert_eq!(m.get(&exec), ["echo", "ls", "x"]);
    let e = cmd.try_parse_args(["-e"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::MissingValue);
}

#[test]
fn delimiter_splits_each_occurrence() {
    let features = Arg::new("features")
        .short('f')
        .value::<String>()
        .many()
        .delimiter(',');
    let ports = Arg::new("ports").value::<u16>().many().delimiter(':');
    let cmd = Command::new("app").arg(&features).arg(&ports);
    let m = cmd
        .try_parse_args(["-f", "a,b", "-f", "c", "--ports", "80:443"])
        .unwrap();
    assert_eq!(m.get(&features), ["a", "b", "c"]);
    assert_eq!(m.get(&ports), [80, 443]);
    let e = cmd.try_parse_args(["--ports", "80:x"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::InvalidValue);
    assert!(e.message().contains("'x'"), "{}", e.message());
}

// ---------------------------------------------------------- conditions

#[test]
fn required_if_eq_and_required_unless() {
    let mode = Arg::new("mode").value::<String>();
    let key = Arg::new("key")
        .value::<String>()
        .required_if_eq(&mode, "secure");
    let stdin = Arg::new("stdin");
    let file = Arg::new("file").value::<String>().required_unless(&stdin);
    let cmd = Command::new("app")
        .arg(&mode)
        .arg(&key)
        .arg(&stdin)
        .arg(&file);
    assert!(cmd.try_parse_args(["--mode", "plain", "--stdin"]).is_ok());
    let e = cmd
        .try_parse_args(["--mode", "secure", "--stdin"])
        .unwrap_err();
    assert_eq!(e.kind(), ErrorKind::MissingRequired);
    assert!(e.message().contains("--key <KEY>"), "{}", e.message());
    let e = cmd.try_parse_args(["--mode", "plain"]).unwrap_err();
    assert!(e.message().contains("--file <FILE>"), "{}", e.message());
    assert!(cmd.try_parse_args(["--file", "x"]).is_ok());
}

#[test]
fn requires_if_and_per_arg_requires_and_conflicts() {
    let format = Arg::new("format").value::<String>();
    let template = Arg::new("template").value::<String>();
    let format = format.requires_if("custom", &template);
    let force = Arg::new("force")
        .requires(&format)
        .conflicts_with(&template);
    let cmd = Command::new("app").arg(&format).arg(&template).arg(&force);
    assert!(cmd.try_parse_args(["--format", "json"]).is_ok());
    let e = cmd.try_parse_args(["--format", "custom"]).unwrap_err();
    assert_eq!(
        e.message(),
        "the argument '--format <FORMAT>' requires '--template <TEMPLATE>', which was not provided"
    );
    let e = cmd.try_parse_args(["--force"]).unwrap_err();
    assert!(e.message().contains("requires '--format <FORMAT>'"));
    let e = cmd
        .try_parse_args(["--force", "--format", "x", "--template", "t"])
        .unwrap_err();
    assert_eq!(e.kind(), ErrorKind::Conflict);
    assert_eq!(
        e.message(),
        "the argument '--force' cannot be used with '--template <TEMPLATE>'"
    );
}

// ------------------------------------------------------ subcommand extras

fn tool() -> Command {
    let verbose = Arg::new("verbose").short('v').global();
    let version_check = Arg::new("verify");
    Command::new("tool")
        .version("1.0")
        .arg(&verbose)
        .arg(&version_check)
        .subcommand(Command::new("install").about("Install"))
        .subcommand(Command::new("info").about("Info"))
        .subcommand(Subcommand::from(Command::new("remove")).visible_alias("rm"))
}

#[cfg(feature = "help")]
#[test]
fn help_subcommand_shows_help_for_the_named_level() {
    let cmd = tool();
    let e = cmd.try_parse_args(["help"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::DisplayHelp);
    assert!(e.message().contains("Usage: tool [OPTIONS] [COMMAND]"));
    let e = cmd.try_parse_args(["help", "install"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::DisplayHelp);
    assert!(
        e.message().contains("Usage: tool install"),
        "{}",
        e.message()
    );
    // Unknown names stop the descent; help for the deepest level found.
    let e = cmd.try_parse_args(["help", "nope"]).unwrap_err();
    assert!(e.message().contains("Usage: tool [OPTIONS] [COMMAND]"));
    // A user-defined `help` subcommand wins.
    let own = Command::new("x").subcommand(Command::new("help"));
    let m = own.try_parse_args(["help"]).unwrap();
    assert_eq!(m.subcommand_name(), Some("help"));
}

#[test]
fn arg_required_else_help() {
    let cmd = tool().arg_required_else_help();
    let e = cmd.try_parse_args([] as [&str; 0]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::HelpOnMissingArgs);
    assert_eq!(e.exit_code(), 2);
    assert!(!e.is_display());
    if cfg!(feature = "help") {
        assert!(e.message().contains("Usage:"), "{}", e.message());
        assert_eq!(e.to_string(), e.message());
    }
    assert!(cmd.try_parse_args(["-v"]).is_ok());
    assert!(cmd.try_parse_args(["info"]).is_ok());
}

#[test]
fn infer_long_args_accepts_unique_prefixes() {
    let cmd = tool().infer_long_args();
    assert!(cmd.try_parse_args(["--verb"]).is_ok());
    assert!(cmd.try_parse_args(["--verbo", "info"]).is_ok());
    // `--ver` matches --verbose and --verify.
    let e = cmd.try_parse_args(["--ver"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownOption);
    // Off by default.
    assert_eq!(
        tool().try_parse_args(["--verb"]).unwrap_err().kind(),
        ErrorKind::UnknownOption
    );
}

#[test]
fn infer_subcommands_accepts_unique_prefixes() {
    let cmd = tool().infer_subcommands();
    assert_eq!(
        cmd.try_parse_args(["inst"]).unwrap().subcommand_name(),
        Some("install")
    );
    assert_eq!(
        cmd.try_parse_args(["r"]).unwrap().subcommand_name(),
        Some("remove")
    );
    let e = cmd.try_parse_args(["in"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownSubcommand);
    assert_eq!(
        tool().try_parse_args(["inst"]).unwrap_err().kind(),
        ErrorKind::UnknownSubcommand
    );
}

#[test]
fn external_subcommands_are_passed_through() {
    let cmd = tool().allow_external_subcommands().subcommand_required();
    let m = cmd.try_parse_args(["-v", "deploy", "--fast", "x"]).unwrap();
    let (name, args) = m.external_subcommand().unwrap();
    assert_eq!(name, "deploy");
    assert_eq!(args, [OsString::from("--fast"), OsString::from("x")]);
    assert!(m.subcommand().is_none());
    let m = cmd.try_parse_args(["info"]).unwrap();
    assert!(m.external_subcommand().is_none());
    assert_eq!(m.subcommand_name(), Some("info"));
    let e = tool()
        .subcommand_required()
        .try_parse_args(["deploy"])
        .unwrap_err();
    assert_eq!(e.kind(), ErrorKind::UnknownSubcommand);
}

#[test]
fn trailing_positional_takes_everything_after_its_first_value() {
    let verbose = Arg::new("verbose").short('v');
    let args = Arg::positional_os("ARGS").many().trailing();
    let cmd = Command::new("run").arg(&verbose).arg(&args);
    let m = cmd
        .try_parse_args(["-v", "prog", "-x", "--y", "z"])
        .unwrap();
    assert!(m.get(&verbose));
    assert_eq!(m.get(&args), ["prog", "-x", "--y", "z"]);
}

#[test]
fn ids_lists_explicit_arguments() {
    let a = Arg::new("a");
    let b = Arg::new("b").value::<u8>().default(1);
    let c = Arg::new("c");
    let cmd = Command::new("app").arg(&a).arg(&b).arg(&c);
    let m = cmd.try_parse_args(["--c", "--a"]).unwrap();
    assert_eq!(m.ids().collect::<Vec<_>>(), ["a", "c"]);
}

// ----------------------------------------------------------- value enums

hasami::value_enum! {
    /// Colour choice
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum When { Auto = "auto", Always = "always", Never = "never" }
}

#[test]
fn value_enum_provides_names_parsing_and_display() {
    assert_eq!(When::names(), ["auto", "always", "never"]);
    assert_eq!("never".parse::<When>(), Ok(When::Never));
    assert_eq!(
        "sometimes".parse::<When>().unwrap_err(),
        "unknown value 'sometimes' (expected one of: auto, always, never)"
    );
    assert_eq!(When::Always.to_string(), "always");

    let color = Arg::new("color").value_enum::<When>().default(When::Auto);
    let mode = Arg::positional_enum::<When>("MODE");
    let cmd = Command::new("app").arg(&color).arg(&mode);
    assert_eq!(
        cmd.find_arg("color").unwrap().possible_values(),
        ["auto", "always", "never"]
    );
    let m = cmd.try_parse_args(["--color", "never", "always"]).unwrap();
    assert_eq!(m.get(&color), When::Never);
    assert_eq!(m.get(&mode), Some(When::Always));
    let e = cmd.try_parse_args(["--color", "sometimes"]).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::InvalidValue);
    assert!(
        e.message()
            .contains("[possible values: auto, always, never]")
    );
}

// ------------------------------------------------------------------ help

#[cfg(feature = "help")]
mod help {
    use super::*;

    fn help_of(cmd: &Command, long: bool) -> String {
        if long {
            cmd.render_help()
        } else {
            cmd.render_short_help()
        }
    }

    #[test]
    fn long_help_and_long_about_only_appear_with_double_dash_help() {
        let name = Arg::new("name")
            .value::<String>()
            .help("Short")
            .long_help("The long explanation");
        let cmd = Command::new("app")
            .about("About")
            .long_about("Long about")
            .arg(&name);
        let short = help_of(&cmd, false);
        let long = help_of(&cmd, true);
        assert!(short.starts_with("About\n"));
        assert!(short.contains("  Short\n"));
        assert!(long.starts_with("Long about\n"));
        assert!(long.contains("  The long explanation\n"));
        let e = cmd.try_parse_args(["-h"]).unwrap_err();
        assert_eq!(e.message(), short);
        let e = cmd.try_parse_args(["--help"]).unwrap_err();
        assert_eq!(e.message(), long);
    }

    #[test]
    fn headings_group_arguments_into_sections() {
        let a = Arg::new("alpha").help("A").help_heading("Tuning");
        let b = Arg::new("beta").help("B");
        let c = Arg::new("gamma").help("C").help_heading("Tuning");
        let cmd = Command::new("app").arg(&a).arg(&b).arg(&c);
        let text = help_of(&cmd, true);
        let options = text.find("Options:").unwrap();
        let tuning = text.find("Tuning:").unwrap();
        assert!(options < tuning);
        assert!(
            text.contains("Tuning:\n      --alpha  A\n      --gamma  C\n"),
            "{text}"
        );
        assert!(text.contains("Options:\n      --beta  B\n"), "{text}");
    }

    #[test]
    fn visible_aliases_are_listed_and_accepted() {
        let color = Arg::new("colour")
            .visible_alias("color")
            .alias("hue")
            .help("Paint");
        let cmd = Command::new("app")
            .arg(&color)
            .subcommand(Subcommand::from(Command::new("remove").about("Drop")).visible_alias("rm"));
        let text = help_of(&cmd, true);
        assert!(text.contains("--colour  Paint [aliases: color]"), "{text}");
        assert!(!text.contains("hue"));
        assert!(text.contains("remove  Drop [aliases: rm]"), "{text}");
        assert!(cmd.try_parse_args(["--color"]).unwrap().get(&color));
        assert!(cmd.try_parse_args(["--hue"]).unwrap().get(&color));
        assert_eq!(
            cmd.try_parse_args(["rm"]).unwrap().subcommand_name(),
            Some("remove")
        );
    }

    #[test]
    fn before_help_and_long_version() {
        let cmd = Command::new("app")
            .version("1.2")
            .long_version("1.2 (build 7)")
            .before_help("Banner")
            .about("About");
        assert!(help_of(&cmd, true).starts_with("Banner\n\nAbout\n\nUsage:"));
        let e = cmd.try_parse_args(["-V"]).unwrap_err();
        assert_eq!(e.message(), "app 1.2");
        let e = cmd.try_parse_args(["--version"]).unwrap_err();
        assert_eq!(e.message(), "app 1.2 (build 7)");
    }

    #[test]
    fn help_is_wrapped_to_the_configured_width() {
        let long = "one two three four five six seven eight nine ten eleven twelve";
        let opt = Arg::new("opt").help(long);
        let cmd = Command::new("app").about(long).arg(&opt).term_width(40);
        let text = help_of(&cmd, true);
        assert!(text.lines().all(|l| l.chars().count() <= 40), "{text}");
        assert!(text.starts_with("one two three four five six seven eight\nnine ten"));
        // Continuation lines of an option's help align with the column.
        assert!(
            text.contains("      --opt   one two three four five\n              six seven"),
            "{text}"
        );
        let unwrapped = help_of(&cmd.clone().term_width(0), true);
        assert!(unwrapped.contains(&["  ", long, "\n"].concat()));
    }

    #[test]
    fn explicit_line_breaks_and_indentation_survive_wrapping() {
        let cmd = Command::new("app")
            .after_help("Examples:\n  app --x\n  app --y")
            .term_width(30);
        let text = help_of(&cmd, true);
        assert!(
            text.ends_with("Examples:\n  app --x\n  app --y\n"),
            "{text}"
        );
    }
}

#[cfg(feature = "color")]
#[test]
fn custom_styles_are_kept_on_the_command() {
    use hasami::Styles;
    let cmd = Command::new("app").styles(Styles::COLORED.with_header("\x1b[35m"));
    assert_eq!(cmd.get_styles().header(), "\x1b[35m");
    assert_eq!(cmd.get_styles().literal(), Styles::COLORED.literal());
}

// ------------------------------------------------------- AI-oriented additions

#[test]
fn examples_are_checked_and_shown() {
    let n = Arg::new("number").short('n').value::<u32>();
    let sub_n = n.clone();
    let cmd = Command::new("app")
        .arg(&n)
        .example("app -n 3")
        .example("app --help")
        .example("app -n 'quoted value' # comment")
        .example("app --number nope   # deliberately wrong")
        .subcommand(
            Command::new("sub")
                .arg(&sub_n)
                .example("app sub --number 4")
                .example("sub --number x"),
        );
    let failed = cmd.check_examples().unwrap_err();
    assert_eq!(failed.len(), 3, "{failed:?}");
    assert!(
        failed[0].starts_with("`app -n 'quoted value'`: invalid value 'quoted value'"),
        "{}",
        failed[0]
    );
    assert!(
        failed[1].starts_with("`app --number nope`: invalid value 'nope'"),
        "{}",
        failed[1]
    );
    assert!(failed[2].starts_with("`sub --number x`:"), "{}", failed[2]);
    assert_eq!(cmd.get_examples().len(), 4);
    #[cfg(feature = "help")]
    {
        let help = cmd.render_help();
        assert!(
            help.contains("\nExamples:\n  app -n 3\n  app --help\n"),
            "{help}"
        );
        // Examples come after the options and before after_help.
        let good = Command::new("ok").example("ok").after_help("Bye");
        assert!(good.check_examples().is_ok());
        assert!(good.render_help().ends_with("Examples:\n  ok\n\nBye\n"));
    }
}

#[test]
fn value_types_are_recorded() {
    use hasami::ValueType;
    use std::path::PathBuf;
    let i = Arg::new("i").value::<u16>();
    let f = Arg::new("f").value::<f64>();
    let b = Arg::new("b").value::<bool>();
    let s = Arg::new("s").value::<String>();
    let p = Arg::positional::<PathBuf>("P");
    let o = Arg::new("o").value_os();
    let w = Arg::new("w").value_enum::<When>();
    let flag = Arg::new("flag");
    assert_eq!(i.def().value_type(), Some(ValueType::Integer));
    assert_eq!(f.def().value_type(), Some(ValueType::Float));
    assert_eq!(b.def().value_type(), Some(ValueType::Boolean));
    assert_eq!(s.def().value_type(), Some(ValueType::String));
    assert_eq!(p.def().value_type(), Some(ValueType::Path));
    assert_eq!(o.def().value_type(), Some(ValueType::String));
    assert_eq!(w.def().value_type(), Some(ValueType::Other));
    assert_eq!(flag.def().value_type(), None);
}

#[test]
fn errors_name_the_argument() {
    let n = Arg::new("number").short('n').value::<u32>();
    let a = Arg::new("a").requires(&n);
    let b = Arg::new("b").conflicts_with(&a);
    let cmd = Command::new("app").arg(&n).arg(&a).arg(&b);
    assert_eq!(
        cmd.try_parse_args(["-n", "x"]).unwrap_err().arg(),
        Some("number")
    );
    assert_eq!(
        cmd.try_parse_args(["-n"]).unwrap_err().arg(),
        Some("number")
    );
    assert_eq!(
        cmd.try_parse_args(["-n1", "-n2"]).unwrap_err().arg(),
        Some("number")
    );
    assert_eq!(
        cmd.try_parse_args(["--a"]).unwrap_err().arg(),
        Some("number")
    );
    // The argument that declares the relation is the one named.
    assert_eq!(
        cmd.try_parse_args(["--a", "-n1", "--b"]).unwrap_err().arg(),
        Some("b")
    );
    assert_eq!(cmd.try_parse_args(["--zzz"]).unwrap_err().arg(), None);
    assert_eq!(ErrorKind::InvalidValue.name(), "invalid_value");
    assert_eq!(ErrorKind::HelpOnMissingArgs.name(), "help_on_missing_args");
}

#[cfg(feature = "json")]
#[test]
fn errors_render_as_json() {
    let n = Arg::new("number").short('n').value::<u32>();
    let cmd = Command::new("app").arg(&n);
    let e = cmd.try_parse_args(["-n", "x\"y"]).unwrap_err();
    let json = e.to_json();
    assert!(json.starts_with(r#"{"kind":"invalid_value","exit_code":2,"arg":"number","message":"invalid value 'x\"y' for '--number <NUMBER>': "#), "{json}");
    assert!(
        json.contains(r#","tip":null,"usage":"app [OPTIONS]"}"#),
        "{json}"
    );
    let e = hasami::Error::custom("boom").with_tip("fix it");
    assert_eq!(
        e.to_json(),
        r#"{"kind":"custom","exit_code":2,"arg":null,"message":"boom","tip":"fix it","usage":null}"#
    );
    #[cfg(feature = "help")]
    {
        let e = cmd.try_parse_args(["--help"]).unwrap_err();
        assert!(
            e.to_json()
                .starts_with(r#"{"kind":"display_help","exit_code":0,"#)
        );
    }
}
