//! Documentation generated from a [`kanna::Command`]: a manpage in roff,
//! Markdown, and a standalone HTML page. Subcommands are documented
//! recursively (lazy ones are built).
//!
//! Hidden arguments and subcommands are omitted, as in `--help`.
//!
//! ```
//! use kanna::{Arg, Command};
//!
//! let n = Arg::new("number").short('n').value::<u32>().default(1).help("How many");
//! let cmd = Command::new("app").version("1.0").about("Do things").arg(&n);
//! let man = kanna_doc::manpage(&cmd);
//! assert!(man.starts_with(".TH \"APP\" \"1\""));
//! let md = kanna_doc::markdown(&cmd);
//! assert!(md.contains("`-n`, `--number <NUMBER>`"));
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt::Write as _;

use kanna::{ArgDef, Command};

/// Options for the manpage header.
#[derive(Clone, Debug, Default)]
pub struct ManOptions {
    /// Manual section (default `1`).
    pub section: Option<String>,
    /// Text for the footer's middle field, typically a date.
    pub date: Option<String>,
    /// Text for the footer's left field, typically the project name.
    pub source: Option<String>,
    /// Text for the header's centre field, typically "User Commands".
    pub manual: Option<String>,
}

/// A manpage in roff (`man` macros) for `cmd`, section 1.
pub fn manpage(cmd: &Command) -> String {
    manpage_with(cmd, &ManOptions::default())
}

/// A manpage with explicit header fields.
pub fn manpage_with(cmd: &Command, opts: &ManOptions) -> String {
    let mut out = String::new();
    let section = opts.section.as_deref().unwrap_or("1");
    let _ = writeln!(
        out,
        ".TH \"{}\" \"{}\" \"{}\" \"{}\" \"{}\"",
        roff(&cmd.name().to_uppercase()),
        roff(section),
        roff(opts.date.as_deref().unwrap_or("")),
        roff(opts.source.as_deref().unwrap_or(&version_line(cmd))),
        roff(opts.manual.as_deref().unwrap_or("User Commands"))
    );
    out.push_str(".SH NAME\n");
    match cmd.get_about() {
        Some(about) => {
            let _ = writeln!(out, "{} \\- {}", roff(cmd.name()), roff(first_line(about)));
        }
        None => {
            let _ = writeln!(out, "{}", roff(cmd.name()));
        }
    }
    out.push_str(".SH SYNOPSIS\n");
    let _ = writeln!(out, "\\fB{}\\fR", roff(&cmd.render_usage()));
    if let Some(long) = cmd.get_long_about().or(cmd.get_about()) {
        out.push_str(".SH DESCRIPTION\n");
        let _ = writeln!(out, "{}", roff_paragraphs(long));
    }
    man_body(&mut out, cmd, cmd.name());
    if !cmd.get_examples().is_empty() {
        out.push_str(".SH EXAMPLES\n");
        for ex in cmd.get_examples() {
            let _ = writeln!(out, ".PP\n\\fB{}\\fR", roff(ex));
        }
    }
    if let Some(after) = cmd.get_after_help() {
        out.push_str(".SH NOTES\n");
        let _ = writeln!(out, "{}", roff_paragraphs(after));
    }
    out
}

fn man_body(out: &mut String, cmd: &Command, path: &str) {
    let positionals: Vec<&ArgDef> = cmd
        .args()
        .iter()
        .filter(|a| a.is_positional() && !a.is_hidden())
        .collect();
    let options: Vec<&ArgDef> = cmd
        .args()
        .iter()
        .filter(|a| !a.is_positional() && !a.is_hidden())
        .collect();
    if !positionals.is_empty() {
        out.push_str(".SH ARGUMENTS\n");
        for a in positionals {
            let _ = writeln!(out, ".TP\n\\fI{}\\fR", roff(&a.display_name()));
            let _ = writeln!(out, "{}", roff(&describe(a)));
        }
    }
    if !options.is_empty() || cmd.has_help_flag() || cmd.has_version_flag() {
        out.push_str(".SH OPTIONS\n");
        for a in options {
            let _ = writeln!(out, ".TP\n{}", man_option(a));
            let _ = writeln!(out, "{}", roff(&describe(a)));
        }
        if cmd.has_help_flag() {
            out.push_str(".TP\n\\fB\\-h\\fR, \\fB\\-\\-help\\fR\nPrint help\n");
        }
        if cmd.has_version_flag() {
            out.push_str(".TP\n\\fB\\-V\\fR, \\fB\\-\\-version\\fR\nPrint version\n");
        }
    }
    let subs: Vec<_> = cmd
        .subcommands()
        .iter()
        .filter(|s| !s.is_hidden())
        .collect();
    if !subs.is_empty() {
        out.push_str(".SH SUBCOMMANDS\n");
        for s in &subs {
            let _ = writeln!(out, ".TP\n\\fB{} {}\\fR", roff(path), roff(s.name()));
            let _ = writeln!(out, "{}", roff(s.summary().unwrap_or("")));
        }
        for s in subs {
            let sub = s.build();
            let sub_path = [path, " ", s.name()].concat();
            let _ = writeln!(out, ".SS \"{}\"", roff(&sub_path));
            let _ = writeln!(out, "\\fB{}\\fR", roff(&sub.render_usage_as(&sub_path)));
            if let Some(about) = sub.get_long_about().or(sub.get_about()) {
                let _ = writeln!(out, ".PP\n{}", roff_paragraphs(about));
            }
            man_body(out, &sub, &sub_path);
        }
    }
}

fn man_option(a: &ArgDef) -> String {
    let mut s = String::new();
    let mut names = Vec::new();
    if let Some(c) = a.short() {
        names.push(format!("\\fB\\-{}\\fR", roff(&c.to_string())));
    }
    if let Some(l) = a.long() {
        names.push(format!("\\fB\\-\\-{}\\fR", roff(l)));
    }
    s.push_str(&names.join(", "));
    if let Some(v) = a.value_name() {
        if a.value_is_optional() {
            let _ = write!(s, "[=\\fI{}\\fR]", roff(v));
        } else {
            let _ = write!(s, " \\fI{}\\fR", roff(v));
        }
        if a.is_many() {
            s.push_str("...");
        }
    }
    s
}

/// Escape text for roff: backslashes, leading dots/quotes, and hyphens.
fn roff(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.chars().enumerate() {
        match c {
            '\\' => out.push_str("\\e"),
            '-' => out.push_str("\\-"),
            '.' if i == 0 => out.push_str("\\&."),
            '\'' if i == 0 => out.push_str("\\&'"),
            c => out.push(c),
        }
    }
    out
}

fn roff_paragraphs(s: &str) -> String {
    let mut out = String::new();
    for (i, para) in s.split("\n\n").enumerate() {
        if i > 0 {
            out.push_str(".PP\n");
        }
        for (j, line) in para.lines().enumerate() {
            if j > 0 {
                out.push_str("\n.br\n");
            }
            out.push_str(&roff(line));
        }
        out.push('\n');
    }
    out.trim_end().to_owned()
}

/// Markdown documentation for `cmd`, one heading level per command depth.
pub fn markdown(cmd: &Command) -> String {
    let mut out = String::new();
    md_command(&mut out, cmd, cmd.name(), 1);
    out
}

fn md_command(out: &mut String, cmd: &Command, path: &str, level: usize) {
    let h = "#".repeat(level);
    let _ = writeln!(out, "{h} {path}\n");
    if let Some(about) = cmd.get_long_about().or(cmd.get_about()) {
        let _ = writeln!(out, "{about}\n");
    }
    let _ = writeln!(out, "```\n{}\n```\n", cmd.render_usage_as(path));
    let positionals: Vec<&ArgDef> = cmd
        .args()
        .iter()
        .filter(|a| a.is_positional() && !a.is_hidden())
        .collect();
    let options: Vec<&ArgDef> = cmd
        .args()
        .iter()
        .filter(|a| !a.is_positional() && !a.is_hidden())
        .collect();
    if !positionals.is_empty() {
        let _ = writeln!(out, "{h}# Arguments\n");
        for a in positionals {
            let _ = writeln!(out, "* `{}`{}", a.display_name(), md_help(a));
        }
        out.push('\n');
    }
    if !options.is_empty() || cmd.has_help_flag() || cmd.has_version_flag() {
        let _ = writeln!(out, "{h}# Options\n");
        for a in options {
            let _ = writeln!(out, "* {}{}", md_option(a), md_help(a));
        }
        if cmd.has_help_flag() {
            out.push_str("* `-h`, `--help`: Print help\n");
        }
        if cmd.has_version_flag() {
            out.push_str("* `-V`, `--version`: Print version\n");
        }
        out.push('\n');
    }
    let subs: Vec<_> = cmd
        .subcommands()
        .iter()
        .filter(|s| !s.is_hidden())
        .collect();
    if !subs.is_empty() {
        let _ = writeln!(out, "{h}# Subcommands\n");
        for s in &subs {
            let _ = writeln!(
                out,
                "* [`{} {}`](#{}): {}",
                path,
                s.name(),
                anchor(path, s.name()),
                s.summary().unwrap_or("")
            );
        }
        out.push('\n');
        for s in subs {
            let sub = s.build();
            md_command(out, &sub, &[path, " ", s.name()].concat(), level + 1);
        }
    }
    if !cmd.get_examples().is_empty() {
        let _ = writeln!(out, "{} Examples\n\n```", "#".repeat(level + 1));
        for ex in cmd.get_examples() {
            let _ = writeln!(out, "{ex}");
        }
        out.push_str("```\n\n");
    }
    if let Some(after) = cmd.get_after_help() {
        let _ = writeln!(out, "{after}\n");
    }
}

fn md_option(a: &ArgDef) -> String {
    let mut names = Vec::new();
    if let Some(c) = a.short() {
        names.push(format!("`-{c}`"));
    }
    if let Some(l) = a.long() {
        let mut n = format!("`--{l}");
        if let Some(v) = a.value_name() {
            if a.value_is_optional() {
                let _ = write!(n, "[=<{v}>]");
            } else {
                let _ = write!(n, " <{v}>");
            }
            if a.is_many() {
                n.push_str("...");
            }
        }
        n.push('`');
        names.push(n);
    }
    names.join(", ")
}

fn md_help(a: &ArgDef) -> String {
    let d = describe(a);
    if d.is_empty() {
        String::new()
    } else {
        format!(": {d}")
    }
}

fn anchor(path: &str, name: &str) -> String {
    [path, "-", name].concat().replace(' ', "-").to_lowercase()
}

/// A standalone HTML page for `cmd`.
pub fn html(cmd: &Command) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n<style>\nbody{{font-family:system-ui,sans-serif;max-width:48rem;margin:2rem auto;padding:0 1rem;line-height:1.5}}\ncode,pre{{font-family:ui-monospace,monospace;background:#f4f4f4;padding:.1em .3em;border-radius:3px}}\npre{{padding:.75em;overflow-x:auto}}\ndt{{font-weight:600;margin-top:.5em}}\ndd{{margin-left:1.5em}}\n</style>\n</head>\n<body>",
        esc(cmd.name())
    );
    html_command(&mut out, cmd, cmd.name(), 1);
    out.push_str("</body>\n</html>\n");
    out
}

fn html_command(out: &mut String, cmd: &Command, path: &str, level: usize) {
    let lv = level.min(6);
    let _ = writeln!(
        out,
        "<h{lv} id=\"{}\">{}</h{lv}>",
        esc(&path.replace(' ', "-")),
        esc(path)
    );
    if let Some(about) = cmd.get_long_about().or(cmd.get_about()) {
        let _ = writeln!(out, "<p>{}</p>", esc(about).replace('\n', "<br>\n"));
    }
    let _ = writeln!(out, "<pre>{}</pre>", esc(&cmd.render_usage_as(path)));
    let positionals: Vec<&ArgDef> = cmd
        .args()
        .iter()
        .filter(|a| a.is_positional() && !a.is_hidden())
        .collect();
    let options: Vec<&ArgDef> = cmd
        .args()
        .iter()
        .filter(|a| !a.is_positional() && !a.is_hidden())
        .collect();
    if !positionals.is_empty() {
        let _ = writeln!(out, "<h{}>Arguments</h{}>\n<dl>", lv + 1, lv + 1);
        for a in positionals {
            let _ = writeln!(
                out,
                "<dt><code>{}</code></dt>\n<dd>{}</dd>",
                esc(&a.display_name()),
                esc(&describe(a))
            );
        }
        out.push_str("</dl>\n");
    }
    if !options.is_empty() || cmd.has_help_flag() || cmd.has_version_flag() {
        let _ = writeln!(out, "<h{}>Options</h{}>\n<dl>", lv + 1, lv + 1);
        for a in options {
            let _ = writeln!(
                out,
                "<dt><code>{}</code></dt>\n<dd>{}</dd>",
                esc(&md_option(a).replace('`', "")),
                esc(&describe(a))
            );
        }
        if cmd.has_help_flag() {
            out.push_str("<dt><code>-h, --help</code></dt>\n<dd>Print help</dd>\n");
        }
        if cmd.has_version_flag() {
            out.push_str("<dt><code>-V, --version</code></dt>\n<dd>Print version</dd>\n");
        }
        out.push_str("</dl>\n");
    }
    let subs: Vec<_> = cmd
        .subcommands()
        .iter()
        .filter(|s| !s.is_hidden())
        .collect();
    if !subs.is_empty() {
        let _ = writeln!(out, "<h{}>Subcommands</h{}>\n<ul>", lv + 1, lv + 1);
        for s in &subs {
            let sub_path = [path, " ", s.name()].concat();
            let _ = writeln!(
                out,
                "<li><a href=\"#{}\"><code>{}</code></a>: {}</li>",
                esc(&sub_path.replace(' ', "-")),
                esc(&sub_path),
                esc(s.summary().unwrap_or(""))
            );
        }
        out.push_str("</ul>\n");
        for s in subs {
            let sub = s.build();
            html_command(out, &sub, &[path, " ", s.name()].concat(), level + 1);
        }
    }
    if !cmd.get_examples().is_empty() {
        let _ = writeln!(out, "<h{0}>Examples</h{0}>\n<pre>", level.min(6) + 1);
        for ex in cmd.get_examples() {
            let _ = writeln!(out, "{}", esc(ex));
        }
        out.push_str("</pre>\n");
    }
    if let Some(after) = cmd.get_after_help() {
        let _ = writeln!(out, "<p>{}</p>", esc(after).replace('\n', "<br>\n"));
    }
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// Help text (the long form, as `--help` shows it) plus the
/// `[default: ..]` style annotations.
fn describe(a: &ArgDef) -> String {
    let mut s = a.long_help().or(a.help()).unwrap_or("").to_owned();
    let mut extra = |t: String| {
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str(&t);
    };
    if let Some(d) = a.default_text() {
        extra(format!("[default: {d}]"));
    }
    if !a.possible_values().is_empty() {
        extra(format!(
            "[possible values: {}]",
            a.possible_values().join(", ")
        ));
    }
    #[cfg(feature = "env")]
    if let Some(v) = a.env() {
        extra(format!("[env: {v}]"));
    }
    if !a.visible_aliases().is_empty() {
        extra(format!("[aliases: {}]", a.visible_aliases().join(", ")));
    }
    s
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("")
}

fn version_line(cmd: &Command) -> String {
    match cmd.get_version() {
        Some(v) => format!("{} {v}", cmd.name()),
        None => cmd.name().to_owned(),
    }
}

/// Extension to render a usage line for a nested path.
trait UsageAs {
    fn render_usage_as(&self, path: &str) -> String;
}

impl UsageAs for Command {
    fn render_usage_as(&self, path: &str) -> String {
        // `render_usage` uses the command's own name; swap in the full path.
        let usage = self.render_usage();
        match usage.strip_prefix(self.name()) {
            Some(rest) => [path, rest].concat(),
            None => usage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanna::Arg;

    fn sample() -> Command {
        let n = Arg::new("number")
            .short('n')
            .value::<u32>()
            .default(1)
            .help("How many");
        let color = Arg::new("color")
            .value::<String>()
            .default_missing("always".to_owned())
            .possible(["auto", "always"]);
        let secret = Arg::new("secret").hidden();
        let file = Arg::positional::<String>("FILE").required().help("Input");
        let force = Arg::new("force").short('f').help("Force it");
        Command::new("app")
            .version("1.0")
            .about("Do things")
            .long_about("Do things.\n\nAt length.")
            .after_help("See also: nothing.")
            .arg(&n)
            .arg(&color)
            .arg(&secret)
            .arg(&file)
            .subcommand(Command::new("run").about("Run it").arg(&force))
            .subcommand(Command::new("hidden").about("No").arg(&force))
    }

    #[test]
    fn manpage_structure() {
        let man = manpage(&sample());
        assert!(man.starts_with(".TH \"APP\" \"1\" \"\" \"app 1.0\" \"User Commands\"\n.SH NAME\napp \\- Do things\n.SH SYNOPSIS\n\\fBapp [OPTIONS] <FILE> [COMMAND]\\fR\n.SH DESCRIPTION\nDo things.\n.PP\nAt length.\n"));
        assert!(man.contains(".SH ARGUMENTS\n.TP\n\\fI<FILE>\\fR\nInput\n"));
        assert!(man.contains(
            ".TP\n\\fB\\-n\\fR, \\fB\\-\\-number\\fR \\fINUMBER\\fR\nHow many [default: 1]\n"
        ));
        assert!(
            man.contains("\\fB\\-\\-color\\fR[=\\fICOLOR\\fR]\n[possible values: auto, always]\n")
        );
        assert!(!man.contains("secret"));
        assert!(man.contains(".SH SUBCOMMANDS\n.TP\n\\fBapp run\\fR\nRun it\n"));
        assert!(man.contains(".SS \"app run\"\n\\fBapp run [OPTIONS]\\fR\n"));
        assert!(man.contains(".SH NOTES\nSee also: nothing.\n"));
        let opts = ManOptions {
            section: Some("8".into()),
            date: Some("2026-09-26".into()),
            source: Some("kanna".into()),
            manual: None,
        };
        assert!(
            manpage_with(&sample(), &opts)
                .starts_with(".TH \"APP\" \"8\" \"2026\\-09\\-26\" \"kanna\" \"User Commands\"")
        );
    }

    #[test]
    fn markdown_structure() {
        let md = markdown(&sample());
        assert!(md.starts_with("# app\n\nDo things.\n\nAt length.\n\n```\napp [OPTIONS] <FILE> [COMMAND]\n```\n\n## Arguments\n\n* `<FILE>`: Input\n\n## Options\n\n* `-n`, `--number <NUMBER>`: How many [default: 1]\n* `--color[=<COLOR>]`: [possible values: auto, always]\n* `-h`, `--help`: Print help\n* `-V`, `--version`: Print version\n\n## Subcommands\n\n* [`app run`](#app-run): Run it\n* [`app hidden`](#app-hidden): No\n\n## app run\n\nRun it\n\n```\napp run [OPTIONS]\n```\n"), "{md}");
        assert!(md.ends_with("See also: nothing.\n\n"));
    }

    #[test]
    fn html_structure() {
        let h = html(&sample());
        assert!(h.starts_with("<!DOCTYPE html>"));
        assert!(h.contains("<h1 id=\"app\">app</h1>"));
        assert!(h.contains("<pre>app [OPTIONS] &lt;FILE&gt; [COMMAND]</pre>"));
        assert!(h.contains(
            "<dt><code>-n, --number &lt;NUMBER&gt;</code></dt>\n<dd>How many [default: 1]</dd>"
        ));
        assert!(h.contains("<h2 id=\"app-run\">app run</h2>"));
        assert!(!h.contains("secret"));
        assert!(h.ends_with("</body>\n</html>\n"));
    }

    #[test]
    fn roff_escaping() {
        assert_eq!(roff("a-b\\c"), "a\\-b\\ec");
        assert_eq!(roff(".x"), "\\&.x");
        assert_eq!(roff("'x"), "\\&'x");
    }
}
