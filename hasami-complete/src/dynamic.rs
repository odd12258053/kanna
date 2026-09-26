//! Runtime completion: the shell passes the words typed so far and the
//! binary answers with candidates.
//!
//! Protocol (set by the scripts from [`generate_dynamic`](crate::generate_dynamic)):
//!
//! * `_HASAMI_COMPLETE` = shell name;
//! * `_HASAMI_COMPLETE_WORDS` = the command line words joined by `U+001F`
//!   (the binary name first, the word being completed last, possibly empty);
//! * `_HASAMI_COMPLETE_INDEX` = index of the word being completed.
//!
//! [`complete_from_env`] checks for these, prints one candidate per line
//! (`value<TAB>help` for shells that show descriptions) and exits.
//!
//! ```no_run
//! use hasami::{Arg, Command};
//!
//! let branch = Arg::new("branch").value::<String>().complete_with(|_| vec!["main".into(), "dev".into()]);
//! let cmd = Command::new("app").arg(&branch);
//! hasami_complete::dynamic::complete_from_env(&cmd); // returns unless completing
//! let matches = cmd.parse();
//! ```

use std::ffi::OsString;

use hasami::Command;

/// One completion candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The text to insert.
    pub value: String,
    /// A short description, shown by shells that support it.
    pub help: String,
}

impl Candidate {
    fn new(value: impl Into<String>, help: impl Into<String>) -> Candidate {
        Candidate {
            value: value.into(),
            help: help.into(),
        }
    }
}

/// Candidates for `words[index]` given the command tree `cmd`. `words[0]` is
/// the binary name. Candidates are already filtered by the prefix typed so
/// far.
pub fn complete(cmd: &Command, words: &[String], index: usize) -> Vec<Candidate> {
    let current = words.get(index).map(String::as_str).unwrap_or("");
    let mut node: std::borrow::Cow<'_, Command> = std::borrow::Cow::Borrowed(cmd);
    let mut i = 1;
    let mut after_double_dash = false;
    let mut positional_seen = 0usize;
    let mut want_value_for: Option<hasami::ArgDef> = None;
    let mut out = Vec::new();
    // Walk the words before the cursor.
    while i < index {
        let w = words[i].as_str();
        i += 1;
        if let Some(def) = want_value_for.take() {
            let _ = def;
            continue;
        }
        if after_double_dash {
            positional_seen += 1;
            continue;
        }
        if w == "--" {
            after_double_dash = true;
            continue;
        }
        if let Some(long) = w.strip_prefix("--") {
            let name = long.split('=').next().unwrap_or(long);
            if !long.contains('=') {
                if let Some(def) = node.args().iter().find(|a| {
                    a.long() == Some(name)
                        || a.aliases()
                            .iter()
                            .chain(a.visible_aliases())
                            .any(|al| al == name)
                }) {
                    if def.takes_value() && !def.value_is_optional() {
                        want_value_for = Some(def.clone());
                    }
                }
            }
            continue;
        }
        if w.len() > 1 && w.starts_with('-') {
            // Short cluster: only the last letter can take a separate value.
            if let Some(c) = w.chars().last() {
                if let Some(def) = node
                    .args()
                    .iter()
                    .find(|a| a.short() == Some(c) || a.short_aliases().contains(&c))
                {
                    if def.takes_value() && !def.value_is_optional() && w.chars().count() == 2 {
                        want_value_for = Some(def.clone());
                    }
                }
            }
            continue;
        }
        let built = node.find_subcommand(w).map(hasami::Subcommand::build);
        if let Some(b) = built {
            node = std::borrow::Cow::Owned(b);
            positional_seen = 0;
            continue;
        }
        positional_seen += 1;
    }
    let node: &Command = &node;

    // A value for the previous option?
    if let Some(def) = want_value_for {
        push_values(&mut out, &def, current);
        return finish(out, current);
    }
    // `--opt=val` being typed.
    if let Some((name, partial)) = current.strip_prefix("--").and_then(|s| s.split_once('=')) {
        if let Some(def) = node.args().iter().find(|a| a.long() == Some(name)) {
            let mut vals = Vec::new();
            push_values(&mut vals, def, partial);
            for v in vals {
                out.push(Candidate::new(format!("--{name}={}", v.value), v.help));
            }
        }
        return finish(out, current);
    }
    if !after_double_dash && current.starts_with('-') {
        for a in node.args() {
            if a.is_hidden() || a.is_positional() {
                continue;
            }
            let help = a
                .help()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_owned();
            if let Some(c) = a.short() {
                out.push(Candidate::new(format!("-{c}"), help.clone()));
            }
            if let Some(l) = a.long() {
                out.push(Candidate::new(format!("--{l}"), help.clone()));
            }
            for l in a.visible_aliases() {
                out.push(Candidate::new(format!("--{l}"), help.clone()));
            }
        }
        // Global options of enclosing commands are accepted too.
        for a in cmd.args() {
            if a.is_global() && !a.is_hidden() && !std::ptr::eq(node, cmd) {
                let help = a
                    .help()
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_owned();
                if let Some(c) = a.short() {
                    out.push(Candidate::new(format!("-{c}"), help.clone()));
                }
                if let Some(l) = a.long() {
                    out.push(Candidate::new(format!("--{l}"), help));
                }
            }
        }
        if node.has_help_flag() {
            out.push(Candidate::new("-h", "Print help"));
            out.push(Candidate::new("--help", "Print help"));
        }
        if node.has_version_flag() {
            out.push(Candidate::new("-V", "Print version"));
            out.push(Candidate::new("--version", "Print version"));
        }
        return finish(out, current);
    }
    if !after_double_dash {
        for s in node.subcommands() {
            if !s.is_hidden() {
                out.push(Candidate::new(s.name(), s.summary().unwrap_or("")));
                for a in s.visible_aliases() {
                    out.push(Candidate::new(a.as_str(), s.summary().unwrap_or("")));
                }
            }
        }
    }
    // Positional values.
    let positionals: Vec<&hasami::ArgDef> =
        node.args().iter().filter(|a| a.is_positional()).collect();
    let slot = positionals
        .get(positional_seen)
        .copied()
        .or_else(|| positionals.last().copied().filter(|a| a.is_many()));
    if let Some(def) = slot {
        push_values(&mut out, def, current);
    }
    finish(out, current)
}

fn push_values(out: &mut Vec<Candidate>, def: &hasami::ArgDef, partial: &str) {
    if let Some(f) = def.completer() {
        for v in f(partial) {
            out.push(Candidate::new(v, ""));
        }
    }
    for v in def.possible_values() {
        out.push(Candidate::new(v.clone(), ""));
    }
}

fn finish(mut out: Vec<Candidate>, current: &str) -> Vec<Candidate> {
    out.retain(|c| c.value.starts_with(current));
    out.dedup_by(|a, b| a.value == b.value);
    out
}

/// If the process was started by a completion script (see the module
/// documentation), print the candidates for `cmd` and exit. Otherwise
/// return immediately. Call this before parsing.
pub fn complete_from_env(cmd: &Command) {
    let Some(shell) = std::env::var_os("_HASAMI_COMPLETE") else {
        return;
    };
    let words = std::env::var_os("_HASAMI_COMPLETE_WORDS").unwrap_or_default();
    let index = std::env::var("_HASAMI_COMPLETE_INDEX")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let output = respond(cmd, &shell, &words, index);
    print!("{output}");
    std::process::exit(0);
}

/// The text printed for a completion request; separated from
/// [`complete_from_env`] for testing.
pub fn respond(
    cmd: &Command,
    shell: &std::ffi::OsStr,
    words: &std::ffi::OsStr,
    index: usize,
) -> String {
    let words: Vec<String> = words
        .to_string_lossy()
        .split('\u{1f}')
        .map(str::to_owned)
        .collect();
    let with_help = matches!(
        shell.to_string_lossy().as_ref(),
        "zsh" | "fish" | "powershell" | "nushell"
    );
    let mut out = String::new();
    for c in complete(cmd, &words, index) {
        out.push_str(&c.value);
        if with_help && !c.help.is_empty() {
            if shell == "zsh" {
                out.push(':');
            } else {
                out.push('\t');
            }
            out.push_str(&c.help);
        }
        out.push('\n');
    }
    out
}

/// Convenience for tests and custom protocols: build the `OsString` word
/// list the scripts send.
pub fn join_words<I, S>(words: I) -> OsString
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out = String::new();
    for (i, w) in words.into_iter().enumerate() {
        if i > 0 {
            out.push('\u{1f}');
        }
        out.push_str(w.as_ref());
    }
    OsString::from(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hasami::Arg;

    fn cmd() -> Command {
        let n = Arg::new("number")
            .short('n')
            .value::<u32>()
            .help("How many");
        let color = Arg::new("color")
            .value::<String>()
            .possible(["auto", "always", "never"]);
        let branch = Arg::new("branch")
            .short('b')
            .value::<String>()
            .complete_with(|p| {
                ["main", "dev", "feature/x"]
                    .iter()
                    .filter(|b| b.starts_with(p))
                    .map(|b| (*b).to_owned())
                    .collect()
            });
        let verbose = Arg::new("verbose").short('v').global().help("More");
        let secret = Arg::new("secret").hidden();
        let file = Arg::positional::<String>("FILE")
            .complete_with(|_| vec!["a.txt".into(), "b.txt".into()]);
        let force = Arg::new("force").short('f').help("Force");
        Command::new("app")
            .version("1.0")
            .arg(&n)
            .arg(&color)
            .arg(&branch)
            .arg(&verbose)
            .arg(&secret)
            .arg(&file)
            .subcommand(Command::new("run").about("Run it").arg(&force).arg(&file))
            .subcommand(hasami::Subcommand::from(Command::new("hidden").arg(&force)).hidden())
    }

    fn values(cmd: &Command, words: &[&str]) -> Vec<String> {
        let w: Vec<String> = words.iter().map(|s| (*s).to_owned()).collect();
        complete(cmd, &w, w.len() - 1)
            .into_iter()
            .map(|c| c.value)
            .collect()
    }

    #[test]
    fn options_and_subcommands_at_root() {
        let c = cmd();
        assert_eq!(
            values(&c, &["app", "--"]),
            vec![
                "--number",
                "--color",
                "--branch",
                "--verbose",
                "--help",
                "--version"
            ]
        );
        assert_eq!(values(&c, &["app", "--c"]), vec!["--color"]);
        assert_eq!(
            values(&c, &["app", "-"]),
            vec![
                "-n",
                "--number",
                "--color",
                "-b",
                "--branch",
                "-v",
                "--verbose",
                "-h",
                "--help",
                "-V",
                "--version"
            ]
        );
        assert_eq!(values(&c, &["app", ""]), vec!["run", "a.txt", "b.txt"]);
        assert_eq!(values(&c, &["app", "r"]), vec!["run"]);
    }

    #[test]
    fn option_values() {
        let c = cmd();
        assert_eq!(
            values(&c, &["app", "--color", ""]),
            vec!["auto", "always", "never"]
        );
        assert_eq!(values(&c, &["app", "--color", "a"]), vec!["auto", "always"]);
        assert_eq!(values(&c, &["app", "--color=n"]), vec!["--color=never"]);
        assert_eq!(values(&c, &["app", "-b", "fe"]), vec!["feature/x"]);
        assert_eq!(
            values(&c, &["app", "--branch", ""]),
            vec!["main", "dev", "feature/x"]
        );
        // A flag does not swallow the next word.
        assert_eq!(
            values(&c, &["app", "-v", ""]),
            vec!["run", "a.txt", "b.txt"]
        );
        // The number option takes a value we cannot guess.
        assert_eq!(values(&c, &["app", "-n", ""]), Vec::<String>::new());
    }

    #[test]
    fn inside_subcommand() {
        let c = cmd();
        assert_eq!(
            values(&c, &["app", "run", "-"]),
            vec!["-f", "--force", "-v", "--verbose", "-h", "--help"]
        );
        assert_eq!(values(&c, &["app", "run", ""]), vec!["a.txt", "b.txt"]);
        assert_eq!(values(&c, &["app", "run", "--force", "b"]), vec!["b.txt"]);
    }

    #[test]
    fn double_dash_stops_options() {
        let c = cmd();
        assert_eq!(values(&c, &["app", "--", "-"]), Vec::<String>::new());
        assert_eq!(values(&c, &["app", "--", ""]), vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn respond_formats_per_shell() {
        let c = cmd();
        let words = join_words(["app", "r"]);
        assert_eq!(respond(&c, "bash".as_ref(), &words, 1), "run\n");
        assert_eq!(respond(&c, "zsh".as_ref(), &words, 1), "run:Run it\n");
        assert_eq!(respond(&c, "fish".as_ref(), &words, 1), "run\tRun it\n");
    }
}
