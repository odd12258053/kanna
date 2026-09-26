//! Shell completion for [`hasami::Command`]s.
//!
//! Two mechanisms, usable together:
//!
//! * **Static scripts** ([`generate`]): a self-contained completion script
//!   for one shell, listing every visible subcommand, option and possible
//!   value. No runtime cooperation needed.
//! * **Dynamic completion** ([`dynamic`]): the script asks the binary for
//!   candidates on every keystroke, so values produced by
//!   [`Arg::complete_with`](hasami::Arg::complete_with) (file names, git
//!   branches, ...) and lazily built subcommands are completed too. The
//!   binary opts in by calling [`dynamic::complete_from_env`] first thing
//!   in `main`.
//!
//! ```
//! use hasami::{Arg, Command};
//! use hasami_complete::Shell;
//!
//! let n = Arg::new("number").short('n').value::<u32>().help("How many");
//! let cmd = Command::new("app").arg(&n);
//! let script = hasami_complete::generate(Shell::Bash, &cmd);
//! assert!(script.contains("--number"));
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;
use std::str::FromStr;

use hasami::{ArgDef, Command};

pub mod dynamic;
mod shells;

/// A supported shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Shell {
    /// GNU bash (`complete -F`).
    Bash,
    /// zsh (`compdef` / `_arguments`).
    Zsh,
    /// fish (`complete -c`).
    Fish,
    /// PowerShell (`Register-ArgumentCompleter`).
    PowerShell,
    /// nushell (`extern` signature).
    Nushell,
}

impl Shell {
    /// Every supported shell.
    pub const ALL: [Shell; 5] = [
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::PowerShell,
        Shell::Nushell,
    ];

    /// The name used on the command line (`bash`, `zsh`, `fish`,
    /// `powershell`, `nushell`).
    pub fn name(self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::PowerShell => "powershell",
            Shell::Nushell => "nushell",
        }
    }

    /// The conventional file name for `bin`'s completion script.
    pub fn file_name(self, bin: &str) -> String {
        match self {
            Shell::Bash => format!("{bin}.bash"),
            Shell::Zsh => format!("_{bin}"),
            Shell::Fish => format!("{bin}.fish"),
            Shell::PowerShell => format!("_{bin}.ps1"),
            Shell::Nushell => format!("{bin}.nu"),
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Shell {
    type Err = String;

    fn from_str(s: &str) -> Result<Shell, String> {
        match s.to_ascii_lowercase().as_str() {
            "bash" => Ok(Shell::Bash),
            "zsh" => Ok(Shell::Zsh),
            "fish" => Ok(Shell::Fish),
            "powershell" | "pwsh" => Ok(Shell::PowerShell),
            "nushell" | "nu" => Ok(Shell::Nushell),
            other => Err(format!(
                "unknown shell '{other}'; expected one of bash, zsh, fish, powershell, nushell"
            )),
        }
    }
}

/// A static completion script for `cmd` in `shell`. The binary name is
/// `cmd`'s name; use [`generate_for`] to override it.
pub fn generate(shell: Shell, cmd: &Command) -> String {
    generate_for(shell, cmd, cmd.name())
}

/// A static completion script for `cmd`, completing the executable `bin`.
pub fn generate_for(shell: Shell, cmd: &Command, bin: &str) -> String {
    let tree = Node::build(cmd, bin);
    match shell {
        Shell::Bash => shells::bash::generate(&tree),
        Shell::Zsh => shells::zsh::generate(&tree),
        Shell::Fish => shells::fish::generate(&tree),
        Shell::PowerShell => shells::powershell::generate(&tree),
        Shell::Nushell => shells::nushell::generate(&tree),
    }
}

/// A script that delegates every completion to the binary at runtime (see
/// [`dynamic`]). The binary must call [`dynamic::complete_from_env`].
pub fn generate_dynamic(shell: Shell, bin: &str) -> String {
    match shell {
        Shell::Bash => shells::bash::dynamic(bin),
        Shell::Zsh => shells::zsh::dynamic(bin),
        Shell::Fish => shells::fish::dynamic(bin),
        Shell::PowerShell => shells::powershell::dynamic(bin),
        Shell::Nushell => shells::nushell::dynamic(bin),
    }
}

/// One option as the generators see it.
pub(crate) struct Opt {
    pub short: Option<char>,
    pub long: Option<String>,
    pub aliases: Vec<String>,
    pub help: String,
    pub takes_value: bool,
    pub value_name: String,
    pub possible: Vec<String>,
}

/// A visible command with its visible options, positionals and children,
/// built eagerly so every generator sees the same tree.
pub(crate) struct Node {
    /// `app`, `app add`, ... as separate words.
    pub path: Vec<String>,
    pub about: String,
    pub opts: Vec<Opt>,
    pub help_flag: bool,
    pub version_flag: bool,
    pub positionals: Vec<Opt>,
    pub subs: Vec<Node>,
    /// Visible aliases of this node's own name.
    pub aliases: Vec<String>,
}

impl Node {
    fn build(cmd: &Command, bin: &str) -> Node {
        Node::build_at(cmd, vec![bin.to_owned()])
    }

    fn build_at(cmd: &Command, path: Vec<String>) -> Node {
        let opt = |a: &ArgDef| Opt {
            short: a.short(),
            long: a.long().map(str::to_owned),
            aliases: a
                .aliases()
                .iter()
                .chain(a.visible_aliases())
                .cloned()
                .collect(),
            help: a
                .help()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_owned(),
            takes_value: a.takes_value(),
            value_name: a.value_name().unwrap_or("VALUE").to_owned(),
            possible: a.possible_values().to_vec(),
        };
        let visible = cmd.args().iter().filter(|a| !a.is_hidden());
        Node {
            about: cmd
                .get_about()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_owned(),
            opts: visible
                .clone()
                .filter(|a| !a.is_positional())
                .map(opt)
                .collect(),
            help_flag: cmd.has_help_flag(),
            version_flag: cmd.has_version_flag(),
            positionals: visible.filter(|a| a.is_positional()).map(opt).collect(),
            subs: cmd
                .subcommands()
                .iter()
                .filter(|s| !s.is_hidden())
                .map(|s| {
                    let mut p = path.clone();
                    p.push(s.name().to_owned());
                    let mut node = Node::build_at(&s.build(), p);
                    node.aliases = s.visible_aliases().to_vec();
                    if node.about.is_empty() {
                        node.about = s
                            .summary()
                            .unwrap_or("")
                            .lines()
                            .next()
                            .unwrap_or("")
                            .to_owned();
                    }
                    node
                })
                .collect(),
            path,
            aliases: Vec::new(),
        }
    }

    /// `app_add` style identifier for shell functions.
    pub fn ident(&self) -> String {
        self.path
            .join("_")
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect()
    }

    /// All words that complete at this level: `--long`, `-s`, subcommand
    /// names, plus `--help`/`--version` when enabled.
    pub fn words(&self) -> Vec<String> {
        let mut w = Vec::new();
        for o in &self.opts {
            if let Some(c) = o.short {
                w.push(format!("-{c}"));
            }
            if let Some(l) = &o.long {
                w.push(format!("--{l}"));
            }
            for a in &o.aliases {
                w.push(format!("--{a}"));
            }
        }
        if self.help_flag {
            w.push("-h".into());
            w.push("--help".into());
        }
        if self.version_flag {
            w.push("-V".into());
            w.push("--version".into());
        }
        for s in &self.subs {
            if let Some(name) = s.path.last() {
                w.push(name.clone());
            }
            w.extend(s.aliases.iter().cloned());
        }
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hasami::Arg;

    pub(crate) fn sample() -> Command {
        let n = Arg::new("number")
            .short('n')
            .value::<u32>()
            .help("How many");
        let color = Arg::new("color")
            .value::<String>()
            .possible(["auto", "always", "never"])
            .help("Colour");
        let secret = Arg::new("secret").hidden();
        let verbose = Arg::new("verbose").short('v').global().help("More");
        let file = Arg::positional::<String>("FILE").help("Input");
        let force = Arg::new("force").short('f').help("Force");
        Command::new("app")
            .version("1.0")
            .about("Do things")
            .arg(&n)
            .arg(&color)
            .arg(&secret)
            .arg(&verbose)
            .arg(&file)
            .subcommand(Command::new("run").about("Run it").arg(&force))
            .subcommand(hasami::Subcommand::from(Command::new("hidden").arg(&force)).hidden())
    }

    #[test]
    fn every_shell_lists_the_visible_words() {
        for shell in Shell::ALL {
            let s = generate(shell, &sample());
            // fish spells long options as `-l number`; every other shell as `--number`.
            for w in [
                "number", "color", "verbose", "run", "help", "version", "force",
            ] {
                assert!(s.contains(w), "{shell}: missing {w}\n{s}");
            }
            assert!(!s.contains("secret"), "{shell} leaks hidden option");
            assert!(!s.contains("hidden"), "{shell} leaks hidden subcommand");
            assert!(s.contains("always"), "{shell}: missing possible value");
        }
    }

    /// The bash script must at least parse, when bash is available.
    #[test]
    fn bash_script_parses() {
        // Skip unless `bash` is a working GNU bash: on Windows runners the
        // name resolves to the WSL launcher, which fails with no distro.
        let probe = std::process::Command::new("bash")
            .args(["-c", "echo \"$BASH_VERSION\""])
            .output();
        match probe {
            Ok(out) if out.status.success() && !out.stdout.trim_ascii().is_empty() => {}
            _ => return,
        }
        let script = generate(Shell::Bash, &sample());
        let dynamic = generate_dynamic(Shell::Bash, "app");
        for s in [script, dynamic] {
            let out = std::process::Command::new("bash")
                .arg("-n")
                .arg("-c")
                .arg(&s)
                .output()
                .expect("bash was runnable a moment ago");
            assert!(
                out.status.success(),
                "{}\n{s}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    #[test]
    fn shell_names_round_trip() {
        for shell in Shell::ALL {
            assert_eq!(shell.name().parse::<Shell>().unwrap(), shell);
        }
        assert_eq!("pwsh".parse::<Shell>().unwrap(), Shell::PowerShell);
        assert_eq!("nu".parse::<Shell>().unwrap(), Shell::Nushell);
        assert!("csh".parse::<Shell>().is_err());
        assert_eq!(Shell::Zsh.file_name("app"), "_app");
        assert_eq!(Shell::Fish.file_name("app"), "app.fish");
    }

    #[test]
    fn dynamic_scripts_reference_the_binary_and_protocol() {
        for shell in Shell::ALL {
            let s = generate_dynamic(shell, "app");
            assert!(s.contains("app"), "{shell}");
            assert!(s.contains("_HASAMI_COMPLETE"), "{shell}");
        }
    }
}
