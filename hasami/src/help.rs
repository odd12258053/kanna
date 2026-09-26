//! Help and usage rendering (feature `help`).

use crate::arg::ArgDef;
use crate::command::Command;
use crate::style::Styles;

/// The usage line without the `Usage:` prefix: `app [OPTIONS] --name <NAME> <INPUT> [COMMAND]`.
pub(crate) fn render_usage(cmd: &Command, path: &str) -> String {
    render_usage_styled(cmd, path, &Styles::PLAIN)
}

pub(crate) fn render_usage_styled(cmd: &Command, path: &str, st: &Styles) -> String {
    let mut s = String::new();
    s.push_str(st.literal());
    s.push_str(path);
    s.push_str(st.reset());
    let mut has_optional_opts = cmd.has_help_flag() || cmd.has_version_flag();
    for a in &cmd.args {
        if !a.hidden && !a.positional && !a.required {
            has_optional_opts = true;
        }
    }
    if has_optional_opts {
        s.push_str(" [OPTIONS]");
    }
    for a in &cmd.args {
        if !a.hidden && !a.positional && a.required {
            s.push(' ');
            s.push_str(&option_display(a, st));
        }
    }
    for a in &cmd.args {
        if !a.hidden && a.positional {
            s.push(' ');
            s.push_str(st.placeholder());
            s.push_str(&positional_usage(a));
            s.push_str(st.reset());
        }
    }
    if !cmd.subcommands.is_empty() {
        s.push_str(if cmd.subcommand_required {
            " <COMMAND>"
        } else {
            " [COMMAND]"
        });
    }
    s
}

/// `--name <NAME>` with the name and placeholder styled separately.
fn option_display(a: &ArgDef, st: &Styles) -> String {
    let plain = a.display_name();
    match plain.find(' ') {
        Some(i) if !st.is_plain() => {
            let (name, value) = plain.split_at(i);
            [
                st.literal(),
                name,
                st.reset(),
                st.placeholder(),
                value,
                st.reset(),
            ]
            .concat()
        }
        _ => plain,
    }
}

/// `<NAME>`, `[NAME]`, `<NAME>...` or `[NAME]...`.
fn positional_usage(a: &ArgDef) -> String {
    let name = a.value_name().unwrap_or(&a.id);
    let mut s = String::with_capacity(name.len() + 5);
    s.push(if a.required { '<' } else { '[' });
    s.push_str(name);
    s.push(if a.required { '>' } else { ']' });
    if a.many {
        s.push_str("...");
    }
    s
}

/// The full help text as printed by `--help`.
pub(crate) fn render_help(cmd: &Command, path: &str) -> String {
    render_help_styled(cmd, path, &Styles::PLAIN)
}

pub(crate) fn render_help_styled(cmd: &Command, path: &str, st: &Styles) -> String {
    let mut out = String::new();
    if let Some(about) = cmd.long_about.as_deref().or(cmd.about.as_deref()) {
        out.push_str(about);
        out.push_str("\n\n");
    }
    out.push_str(st.header());
    out.push_str("Usage:");
    out.push_str(st.reset());
    out.push(' ');
    out.push_str(&render_usage_styled(cmd, path, st));
    out.push('\n');

    let mut subs = Vec::new();
    for s in &cmd.subcommands {
        if !s.hidden {
            subs.push(Row {
                literal: s.name.clone(),
                placeholder: String::new(),
                right: s.about.clone().unwrap_or_default(),
            });
        }
    }
    let mut positionals = Vec::new();
    let mut options = Vec::new();
    for a in &cmd.args {
        if a.hidden {
            continue;
        }
        if a.positional {
            positionals.push(Row {
                literal: String::new(),
                placeholder: positional_usage(a),
                right: describe(a),
            });
        } else {
            let (literal, placeholder) = option_left(a);
            options.push(Row {
                literal,
                placeholder,
                right: describe(a),
            });
        }
    }
    if cmd.has_help_flag() {
        options.push(Row::literal("-h, --help", "Print help"));
    }
    if cmd.has_version_flag() {
        options.push(Row::literal("-V, --version", "Print version"));
    }

    section(&mut out, "Commands", &subs, st);
    section(&mut out, "Arguments", &positionals, st);
    section(&mut out, "Options", &options, st);

    if let Some(after) = &cmd.after_help {
        out.push('\n');
        out.push_str(after);
        out.push('\n');
    }
    out
}

/// One entry of a help section: the literal part of the left column (option
/// or subcommand name), the placeholder part (`<VALUE>`), and the help text.
struct Row {
    literal: String,
    placeholder: String,
    right: String,
}

impl Row {
    fn literal(left: &str, right: &str) -> Row {
        Row {
            literal: left.to_owned(),
            placeholder: String::new(),
            right: right.to_owned(),
        }
    }

    fn width(&self) -> usize {
        self.literal.chars().count() + self.placeholder.chars().count()
    }
}

/// `-n, --name` / `    --name` / `-n`, plus ` <NAME>` / `[=<NAME>]` / `...`.
fn option_left(a: &ArgDef) -> (String, String) {
    let mut s = String::new();
    match (a.short, &a.long) {
        (Some(c), Some(l)) => {
            s.push('-');
            s.push(c);
            s.push_str(", --");
            s.push_str(l);
        }
        (Some(c), None) => {
            s.push('-');
            s.push(c);
        }
        (None, Some(l)) => {
            s.push_str("    --");
            s.push_str(l);
        }
        (None, None) => s.push_str(&a.id),
    }
    let mut v = String::new();
    if let Some(vd) = &a.value {
        if vd.default_missing.is_some() {
            v.push_str("[=<");
            v.push_str(&vd.name);
            v.push_str(">]");
        } else {
            v.push_str(" <");
            v.push_str(&vd.name);
            v.push('>');
        }
        if a.many {
            v.push_str("...");
        }
    }
    (s, v)
}

/// Help text plus `[default: ..]`, `[possible values: ..]`, `[env: ..]`.
fn describe(a: &ArgDef) -> String {
    let mut s = a.help.clone().unwrap_or_default();
    let mut extra = |text: String| {
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str(&text);
    };
    if let Some(v) = &a.value {
        if let Some((_, d)) = &v.default {
            extra(["[default: ", d, "]"].concat());
        }
        if !v.possible.is_empty() {
            extra(["[possible values: ", &v.possible.join(", "), "]"].concat());
        }
        #[cfg(feature = "env")]
        if let Some(var) = &v.env {
            extra(["[env: ", var, "]"].concat());
        }
    }
    s
}

const INDENT: usize = 2;
const MAX_LEFT: usize = 36;

fn section(out: &mut String, title: &str, rows: &[Row], st: &Styles) {
    if rows.is_empty() {
        return;
    }
    out.push('\n');
    out.push_str(st.header());
    out.push_str(title);
    out.push(':');
    out.push_str(st.reset());
    out.push('\n');
    let mut widest = 0;
    for row in rows {
        let w = row.width();
        if w <= MAX_LEFT && w > widest {
            widest = w;
        }
    }
    let col = INDENT + widest + 2;
    for row in rows {
        pad(out, INDENT);
        out.push_str(st.literal());
        out.push_str(&row.literal);
        out.push_str(st.reset());
        out.push_str(st.placeholder());
        out.push_str(&row.placeholder);
        out.push_str(st.reset());
        if row.right.is_empty() {
            out.push('\n');
            continue;
        }
        let width = row.width();
        let mut lines = row.right.lines();
        if width > MAX_LEFT {
            out.push('\n');
            pad(out, col);
        } else {
            pad(out, col - INDENT - width);
        }
        if let Some(first) = lines.next() {
            out.push_str(first);
        }
        out.push('\n');
        for line in lines {
            pad(out, col);
            out.push_str(line);
            out.push('\n');
        }
    }
}

fn pad(out: &mut String, n: usize) {
    for _ in 0..n {
        out.push(' ');
    }
}
