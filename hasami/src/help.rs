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

/// The full help text: the `--help` form when `long`, the `-h` form
/// otherwise. They differ only where `long_about` or `long_help` are set.
pub(crate) fn render_help(cmd: &Command, path: &str, long: bool) -> String {
    render_help_styled(cmd, path, &Styles::PLAIN, long)
}

/// The width help is wrapped to: the command's setting, else `COLUMNS`,
/// else 100. Zero means no wrapping.
fn width_of(cmd: &Command) -> usize {
    cmd.term_width.unwrap_or_else(|| {
        std::env::var("COLUMNS")
            .ok()
            .and_then(|c| c.parse().ok())
            .unwrap_or(100)
    })
}

pub(crate) fn render_help_styled(cmd: &Command, path: &str, st: &Styles, long: bool) -> String {
    let width = width_of(cmd);
    let mut out = String::new();
    if let Some(before) = &cmd.before_help {
        out.push_str(&wrap(before, width));
        out.push_str("\n\n");
    }
    let about = if long {
        cmd.long_about.as_deref().or(cmd.about.as_deref())
    } else {
        cmd.about.as_deref()
    };
    if let Some(about) = about {
        out.push_str(&wrap(about, width));
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
            let mut right = s.about.clone().unwrap_or_default();
            aliases_note(&mut right, &s.visible_aliases);
            subs.push(Row {
                literal: s.name.clone(),
                placeholder: String::new(),
                right,
            });
        }
    }
    // Sections in order: Arguments, Options, then custom headings as they
    // first appear.
    let mut sections: Vec<(&str, Vec<Row>)> =
        vec![("Arguments", Vec::new()), ("Options", Vec::new())];
    for a in &cmd.args {
        if a.hidden {
            continue;
        }
        let row = if a.positional {
            Row {
                literal: String::new(),
                placeholder: positional_usage(a),
                right: describe(a, long),
            }
        } else {
            let (literal, placeholder) = option_left(a);
            Row {
                literal,
                placeholder,
                right: describe(a, long),
            }
        };
        let title =
            a.heading
                .as_deref()
                .unwrap_or(if a.positional { "Arguments" } else { "Options" });
        match sections.iter_mut().find(|(t, _)| *t == title) {
            Some((_, rows)) => rows.push(row),
            None => sections.push((title, vec![row])),
        }
    }
    if cmd.has_help_flag() {
        sections[1].1.push(Row::literal("-h, --help", "Print help"));
    }
    if cmd.has_version_flag() {
        sections[1]
            .1
            .push(Row::literal("-V, --version", "Print version"));
    }

    section(&mut out, "Commands", &subs, st, width);
    for (title, rows) in &sections {
        section(&mut out, title, rows, st, width);
    }

    if let Some(after) = &cmd.after_help {
        out.push('\n');
        out.push_str(&wrap(after, width));
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

/// Append `[aliases: a, b]` to a help text.
fn aliases_note(s: &mut String, aliases: &[String]) {
    if aliases.is_empty() {
        return;
    }
    if !s.is_empty() {
        s.push(' ');
    }
    s.push_str("[aliases: ");
    s.push_str(&aliases.join(", "));
    s.push(']');
}

/// Help text plus `[default: ..]`, `[possible values: ..]`, `[env: ..]`,
/// `[aliases: ..]`.
fn describe(a: &ArgDef, long: bool) -> String {
    let base = if long {
        a.long_help.as_deref().or(a.help.as_deref())
    } else {
        a.help.as_deref()
    };
    let mut s = base.unwrap_or_default().to_owned();
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
    aliases_note(&mut s, &a.visible_aliases);
    s
}

const INDENT: usize = 2;
const MAX_LEFT: usize = 36;

fn section(out: &mut String, title: &str, rows: &[Row], st: &Styles, width: usize) {
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
    // Never wrap the right column narrower than this, whatever the width.
    let right_width = if width == 0 {
        0
    } else {
        width.saturating_sub(col).max(20)
    };
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
        let left = row.width();
        let text = wrap(&row.right, right_width);
        let mut lines = text.lines();
        if left > MAX_LEFT {
            out.push('\n');
            pad(out, col);
        } else {
            pad(out, col - INDENT - left);
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

/// Word-wrap `text` to `width` columns, keeping explicit line breaks and
/// runs of spaces (indentation). A width of zero returns the text unchanged.
fn wrap(text: &str, width: usize) -> String {
    if width == 0 {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len() + 8);
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut used = 0;
        let mut need_sep = false;
        for word in line.split(' ') {
            if word.is_empty() {
                out.push(' ');
                used += 1;
                need_sep = false;
                continue;
            }
            let w = word.chars().count();
            if need_sep {
                if used + 1 + w > width {
                    out.push('\n');
                    used = 0;
                } else {
                    out.push(' ');
                    used += 1;
                }
            }
            out.push_str(word);
            used += w;
            need_sep = true;
        }
    }
    out
}
