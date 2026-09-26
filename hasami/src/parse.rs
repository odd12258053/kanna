//! The parse engine: drives a [`hasami_core::Parser`] according to a
//! [`Command`] and fills [`Matches`].

use std::ffi::{OsStr, OsString};

use hasami_core::{Arg as Tok, Parser};

use crate::arg::{ArgDef, ParseFailure, ValueDef};
use crate::command::{CmdRef, Command};
use crate::error::{Error, ErrorKind};
use crate::matches::{Matches, Slot, Source, Stored};

/// One level of the command tree during parsing.
struct Frame<'a> {
    cmd: CmdRef<'a>,
    matches: Matches,
    /// Canonical subcommand name, for the parent's `Matches::subcommand`.
    name: String,
    /// `app sub subsub`, for usage and help.
    path: String,
}

impl<'a> Frame<'a> {
    #[inline(never)]
    fn new(cmd: CmdRef<'a>, name: String, path: String) -> Frame<'a> {
        let matches = Matches::new(cmd.args.iter().map(|a| a.id.clone()));
        Frame {
            cmd,
            matches,
            name,
            path,
        }
    }

    fn usage(&self) -> String {
        #[cfg(feature = "help")]
        {
            crate::help::render_usage(&self.cmd, &self.path)
        }
        #[cfg(not(feature = "help"))]
        {
            self.path.clone()
        }
    }
}

enum Name {
    Short(char),
    Long(String),
}

impl Name {
    fn display(&self) -> String {
        match self {
            Name::Short(c) => {
                let mut s = String::from("-");
                s.push(*c);
                s
            }
            Name::Long(l) => msg(&["--", l]),
        }
    }
}

/// Quote a raw value for a message.
pub(crate) fn q(v: &OsStr) -> String {
    ["'", &v.to_string_lossy(), "'"].concat()
}

/// Build a message from pieces. Cheaper in code size than `format!` at
/// every call site, which matters because error paths are numerous.
fn msg(parts: &[&str]) -> String {
    parts.concat()
}

pub(crate) fn parse(root: &Command, p: &mut Parser) -> Result<Matches, Error> {
    let mut frames = vec![Frame::new(
        CmdRef::Borrowed(root),
        root.name.clone(),
        root.name.clone(),
    )];
    match run(&mut frames, p) {
        Ok(()) => Ok(nest(frames)),
        Err(mut e) => {
            if let Some(top) = frames.last() {
                e.set_usage_if_missing(|| top.usage());
                e = e.with_help_context(&top.cmd, &top.path);
            }
            Err(e)
        }
    }
}

fn run(frames: &mut Vec<Frame<'_>>, p: &mut Parser) -> Result<(), Error> {
    loop {
        // `Arg::Long` borrows the parser, so copy the name out before using
        // the parser again.
        let tok = match p.next() {
            Ok(Some(Tok::Short(c))) => Name::Short(c),
            Ok(Some(Tok::Long(l))) => Name::Long(l.to_owned()),
            Ok(Some(Tok::Value(v))) => {
                value(frames, v)?;
                continue;
            }
            Ok(None) => break,
            Err(e) => return Err(convert_core(e, frames)),
        };
        option(frames, p, tok)?;
    }
    finish(frames)
}

fn convert_core(e: hasami_core::Error, frames: &[Frame<'_>]) -> Error {
    use hasami_core::Error as Core;
    match e {
        Core::UnexpectedValue { option, value } => {
            let name = find_by_spelling(frames, &option)
                .map(|d| d.display_name())
                .unwrap_or(option);
            Error::new(
                ErrorKind::UnexpectedValue,
                msg(&[
                    "unexpected value ",
                    &q(&value),
                    " for '",
                    &name,
                    "', which takes no value",
                ]),
            )
        }
        Core::UnexpectedOption(o) => Error::new(
            ErrorKind::UnknownOption,
            msg(&["unexpected argument '", &o, "' found"]),
        ),
        // Only `optional_value` on an exotic platform can produce this.
        other => Error::new(ErrorKind::NonUnicode, other.to_string()),
    }
}

/// Find an argument by how it was spelled (`-n`, `--name`).
fn find_by_spelling<'f>(frames: &'f [Frame<'_>], spelling: &str) -> Option<&'f ArgDef> {
    let name = if let Some(l) = spelling.strip_prefix("--") {
        Name::Long(l.to_owned())
    } else {
        Name::Short(spelling.strip_prefix('-')?.chars().next()?)
    };
    lookup(frames, &name).map(|(fi, ai)| &frames[fi].cmd.args[ai])
}

/// Locate an option: the innermost command first, then global options of
/// the enclosing commands.
fn lookup(frames: &[Frame<'_>], name: &Name) -> Option<(usize, usize)> {
    let last = frames.len().checked_sub(1)?;
    for fi in (0..=last).rev() {
        let global_only = fi != last;
        let hit = frames[fi].cmd.args.iter().position(|a| {
            !a.positional
                && (!global_only || a.global)
                && match name {
                    Name::Short(c) => a.matches_short(*c),
                    Name::Long(l) => a.matches_long(l),
                }
        });
        if let Some(ai) = hit {
            return Some((fi, ai));
        }
    }
    None
}

fn option(frames: &mut [Frame<'_>], p: &mut Parser, name: Name) -> Result<(), Error> {
    let Some((fi, ai)) = lookup(frames, &name) else {
        return Err(unknown_option(frames, &name));
    };
    let frame = &mut frames[fi];
    let def = &frame.cmd.args[ai];
    let slot = &mut frame.matches.slots[ai];
    match &def.value {
        None => {
            slot.values.push(Stored {
                raw: OsString::new(),
                value: Box::new(()),
            });
            slot.source = Some(Source::CommandLine);
            Ok(())
        }
        Some(vd) => {
            let raw = if let Some((make, text)) = &vd.default_missing {
                match p.optional_value()? {
                    Some(v) => v,
                    None => {
                        slot.values.push(Stored {
                            raw: OsString::from(text),
                            value: make(),
                        });
                        slot.source = Some(Source::CommandLine);
                        return Ok(());
                    }
                }
            } else {
                p.value().map_err(|_| missing_value(def))?
            };
            store(def, vd, raw, slot, Source::CommandLine)
        }
    }
}

fn missing_value(def: &ArgDef) -> Error {
    Error::new(
        ErrorKind::MissingValue,
        msg(&[
            "a value is required for '",
            &def.display_name(),
            "' but none was supplied",
        ]),
    )
}

fn unknown_option(frames: &[Frame<'_>], name: &Name) -> Error {
    let display = name.display();
    #[cfg(feature = "help")]
    if let Some(top) = frames.last() {
        let is_help =
            matches!(name, Name::Long(l) if l == "help") || matches!(name, Name::Short('h'));
        if top.cmd.has_help_flag() && is_help {
            return Error::new(
                ErrorKind::DisplayHelp,
                crate::help::render_help(&top.cmd, &top.path),
            );
        }
        let is_version =
            matches!(name, Name::Long(l) if l == "version") || matches!(name, Name::Short('V'));
        if top.cmd.has_version_flag() && is_version {
            let version = top.cmd.version.as_deref().unwrap_or_default();
            return Error::new(
                ErrorKind::DisplayVersion,
                msg(&[&top.cmd.name, " ", version]),
            );
        }
    }
    let mut e = Error::new(
        ErrorKind::UnknownOption,
        msg(&["unexpected argument '", &display, "' found"]),
    );
    if matches!(name, Name::Short('\u{FFFD}')) {
        return e.with_tip("the option letter is not valid unicode");
    }
    #[cfg(feature = "suggest")]
    {
        let candidates = option_spellings(frames);
        if let Some(best) = crate::suggest::closest(&display, candidates.iter().map(String::as_str))
        {
            e = e.with_tip(msg(&["a similar argument exists: '", best, "'"]));
            return e;
        }
    }
    if let Some(top) = frames.last() {
        if top.cmd.args.iter().any(|a| a.positional) {
            e = e.with_tip(msg(&[
                "to pass '",
                &display,
                "' as a value, use '-- ",
                &display,
                "'",
            ]));
        }
    }
    e
}

#[cfg(feature = "suggest")]
fn option_spellings(frames: &[Frame<'_>]) -> Vec<String> {
    let mut out = Vec::new();
    let last = frames.len().saturating_sub(1);
    for (fi, frame) in frames.iter().enumerate() {
        for a in &frame.cmd.args {
            if !a.positional && !a.hidden && (fi == last || a.global) {
                out.extend(a.spellings());
            }
        }
    }
    if let Some(top) = frames.last() {
        if top.cmd.has_help_flag() {
            out.push("--help".to_owned());
        }
        if top.cmd.has_version_flag() {
            out.push("--version".to_owned());
        }
    }
    out
}

fn value(frames: &mut Vec<Frame<'_>>, v: OsString) -> Result<(), Error> {
    let Some(top) = frames.last_mut() else {
        return Ok(());
    };
    if !top.cmd.subcommands.is_empty() {
        if let Some(sub) = v.to_str().and_then(|n| top.cmd.find_subcommand(n)) {
            let path = msg(&[&top.path, " ", &sub.name]);
            let name = sub.name.clone();
            let cmd = sub.resolve();
            frames.push(Frame::new(cmd, name, path));
            return Ok(());
        }
    }
    let positional = top
        .cmd
        .args
        .iter()
        .enumerate()
        .find(|(ai, a)| a.positional && (a.many || top.matches.slots[*ai].values.is_empty()))
        .map(|(ai, _)| ai);
    let Some(ai) = positional else {
        return Err(unexpected_value(frames, &v));
    };
    let def = &top.cmd.args[ai];
    let Some(vd) = &def.value else {
        return Ok(());
    };
    store(def, vd, v, &mut top.matches.slots[ai], Source::CommandLine)
}

fn unexpected_value(frames: &[Frame<'_>], v: &OsStr) -> Error {
    let unexpected = || {
        Error::new(
            ErrorKind::UnexpectedArgument,
            msg(&["unexpected argument ", &q(v), " found"]),
        )
    };
    let Some(top) = frames.last() else {
        return unexpected();
    };
    if !top.cmd.subcommands.is_empty() {
        let e = Error::new(
            ErrorKind::UnknownSubcommand,
            msg(&["unrecognized subcommand ", &q(v)]),
        );
        #[cfg(feature = "suggest")]
        if let Some(name) = v.to_str() {
            let names = top
                .cmd
                .subcommands
                .iter()
                .filter(|s| !s.hidden)
                .map(|s| s.name.as_str());
            if let Some(best) = crate::suggest::closest(name, names) {
                return e.with_tip(msg(&["a similar subcommand exists: '", best, "'"]));
            }
        }
        if top.cmd.has_help_flag() {
            return e.with_tip(msg(&[
                "see '",
                &top.path,
                " --help' for the list of subcommands",
            ]));
        }
        return e;
    }
    let e = unexpected();
    let mut count = 0usize;
    let mut last: Option<&ArgDef> = None;
    for a in &top.cmd.args {
        if a.positional {
            count += 1;
            last = Some(a);
        }
    }
    if let Some(last) = last {
        e.with_tip(msg(&[
            "'",
            &last.display_name(),
            "' was already given; the command accepts ",
            &count.to_string(),
            " positional argument",
            if count == 1 { "" } else { "s" },
        ]))
    } else {
        e.with_tip("the command takes no positional arguments")
    }
}

#[inline(never)]
fn store(
    def: &ArgDef,
    vd: &ValueDef,
    raw: OsString,
    slot: &mut Slot,
    source: Source,
) -> Result<(), Error> {
    if !vd.possible.is_empty() {
        let ok = raw
            .to_str()
            .is_some_and(|s| vd.possible.iter().any(|p| p == s));
        if !ok {
            return Err(not_possible(def, vd, &raw));
        }
    }
    match (vd.parser)(&raw) {
        Ok(value) => {
            slot.values.push(Stored { raw, value });
            slot.source = Some(source);
            Ok(())
        }
        Err(failure) => Err(invalid_value(def, &raw, failure)),
    }
}

fn not_possible(def: &ArgDef, vd: &ValueDef, raw: &OsStr) -> Error {
    #[allow(unused_mut)]
    let mut e = Error::new(
        ErrorKind::InvalidValue,
        msg(&[
            "invalid value ",
            &q(raw),
            " for '",
            &def.display_name(),
            "'\n  [possible values: ",
            &vd.possible.join(", "),
            "]",
        ]),
    );
    #[cfg(feature = "suggest")]
    if let Some(s) = raw.to_str() {
        if let Some(best) = crate::suggest::closest(s, vd.possible.iter().map(String::as_str)) {
            e = e.with_tip(msg(&["a similar value exists: '", best, "'"]));
        }
    }
    e
}

fn invalid_value(def: &ArgDef, raw: &OsStr, failure: ParseFailure) -> Error {
    let (kind, reason) = match failure {
        ParseFailure::NonUnicode => (ErrorKind::NonUnicode, String::from("not valid unicode")),
        ParseFailure::Invalid(reason) => (ErrorKind::InvalidValue, reason),
    };
    Error::new(
        kind,
        msg(&[
            "invalid value ",
            &q(raw),
            " for '",
            &def.display_name(),
            "': ",
            &reason,
        ]),
    )
}

fn finish(frames: &mut [Frame<'_>]) -> Result<(), Error> {
    let last = frames.len().saturating_sub(1);
    for (fi, frame) in frames.iter_mut().enumerate() {
        if let Err(e) = finish_frame(frame, fi == last) {
            let mut e = e;
            e.set_usage_if_missing(|| frame.usage());
            return Err(e.with_help_context(&frame.cmd, &frame.path));
        }
    }
    Ok(())
}

fn finish_frame(frame: &mut Frame<'_>, is_leaf: bool) -> Result<(), Error> {
    let cmd = &*frame.cmd;
    let matches = &mut frame.matches;

    // Environment fallback and defaults.
    for (ai, def) in cmd.args.iter().enumerate() {
        let slot = &mut matches.slots[ai];
        if !slot.values.is_empty() {
            continue;
        }
        let Some(vd) = &def.value else {
            continue;
        };
        #[cfg(feature = "env")]
        if let Some(var) = &vd.env {
            if let Some(v) = std::env::var_os(var) {
                store(def, vd, v, slot, Source::Env)?;
                continue;
            }
        }
        if let Some((make, text)) = &vd.default {
            slot.values.push(Stored {
                raw: OsString::from(text),
                value: make(),
            });
            slot.source = Some(Source::Default);
        }
    }

    // Required arguments, reported together.
    let mut text = String::new();
    for (ai, a) in cmd.args.iter().enumerate() {
        if a.required && matches.slots[ai].values.is_empty() {
            text.push_str("\n  ");
            text.push_str(&a.display_name());
        }
    }
    if !text.is_empty() {
        text.insert_str(0, "the following required arguments were not provided:");
        return Err(Error::new(ErrorKind::MissingRequired, text));
    }

    // Groups.
    for g in &cmd.groups {
        let present: Vec<&ArgDef> = g
            .members
            .iter()
            .filter(|id| matches.contains_id(id))
            .filter_map(|id| cmd.find_arg(id))
            .collect();
        if let (true, [a, b, ..]) = (g.exclusive, present.as_slice()) {
            return Err(Error::new(
                ErrorKind::Conflict,
                msg(&[
                    "the argument '",
                    &a.display_name(),
                    "' cannot be used with '",
                    &b.display_name(),
                    "'",
                ]),
            ));
        }
        if g.required && present.is_empty() {
            let names: Vec<String> = g
                .members
                .iter()
                .filter_map(|id| cmd.find_arg(id))
                .map(|a| a.display_name())
                .collect();
            return Err(Error::new(
                ErrorKind::MissingRequired,
                msg(&[
                    "one of the following arguments is required: ",
                    &names.join(", "),
                ]),
            ));
        }
    }

    // Requires.
    for (a, b) in &cmd.requires {
        if matches.contains_id(a) && !matches.contains_id(b) {
            let name = |id: &str| {
                cmd.find_arg(id)
                    .map(ArgDef::display_name)
                    .unwrap_or_else(|| id.to_owned())
            };
            return Err(Error::new(
                ErrorKind::MissingRequired,
                msg(&[
                    "the argument '",
                    &name(a),
                    "' requires '",
                    &name(b),
                    "', which was not provided",
                ]),
            ));
        }
    }

    if is_leaf && cmd.subcommand_required && !cmd.subcommands.is_empty() {
        let names: Vec<&str> = cmd
            .subcommands
            .iter()
            .filter(|s| !s.hidden)
            .map(|s| s.name.as_str())
            .collect();
        return Err(Error::new(
            ErrorKind::MissingSubcommand,
            "a subcommand is required but one was not provided",
        )
        .with_tip(msg(&["available subcommands: ", &names.join(", ")])));
    }
    Ok(())
}

fn nest(mut frames: Vec<Frame<'_>>) -> Matches {
    while frames.len() > 1 {
        let Some(top) = frames.pop() else { break };
        if let Some(parent) = frames.last_mut() {
            parent.matches.sub = Some(Box::new((top.name, top.matches)));
        }
    }
    frames
        .pop()
        .map(|f| f.matches)
        .unwrap_or_else(|| Matches::new(Vec::new()))
}
