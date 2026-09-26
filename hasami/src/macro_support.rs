//! Runtime support for the [`cli!`](crate::cli) macro. Not part of the
//! public API; everything here may change without notice.

use std::any::Any;
use std::ffi::OsString;
use std::fmt::Display;
use std::str::FromStr;

use crate::arg::ArgDef;
use crate::{Arg, Command, Matches, Subcommand};

/// Settings gathered from a field's attributes and doc comment, turned into
/// a typed [`Arg`] by one of the finalisers.
///
/// Everything that does not depend on `T` lives in the non-generic
/// [`Common`] so that it is compiled once, not once per value type.
pub struct FieldSpec<T> {
    common: Common,
    default: Option<(T, String)>,
    default_missing: Option<(T, String)>,
}

/// The type-independent part of a [`FieldSpec`].
pub struct Common {
    field: &'static str,
    long: Option<String>,
    short: Option<char>,
    aliases: Vec<&'static str>,
    short_aliases: Vec<char>,
    help: String,
    help_override: Option<&'static str>,
    hidden: bool,
    global: bool,
    positional: bool,
    count: bool,
    required: bool,
    value_name: Option<&'static str>,
    #[cfg(feature = "env")]
    env: Option<&'static str>,
    possible: &'static [&'static str],
}

/// `snake_case` to `kebab-case`.
pub fn kebab(s: &str) -> String {
    s.replace('_', "-")
}

/// `CamelCase` to `kebab-case` (for enum variants).
pub fn camel_kebab(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c == '_' {
            out.push('-');
        } else if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

impl Common {
    fn new(field: &'static str) -> Common {
        Common {
            field,
            long: None,
            short: None,
            aliases: Vec::new(),
            short_aliases: Vec::new(),
            help: String::new(),
            help_override: None,
            hidden: false,
            global: false,
            positional: false,
            count: false,
            required: false,
            value_name: None,
            #[cfg(feature = "env")]
            env: None,
            possible: &[],
        }
    }

    fn long_name(&self) -> String {
        self.long.clone().unwrap_or_else(|| kebab(self.field))
    }

    fn upper_name(&self) -> String {
        self.value_name
            .map(str::to_owned)
            .unwrap_or_else(|| self.field.to_ascii_uppercase())
    }

    fn help_text(&self) -> Option<String> {
        if let Some(h) = self.help_override {
            return Some(h.to_owned());
        }
        if self.help.is_empty() {
            None
        } else {
            Some(self.help.clone())
        }
    }

    /// Apply every type-independent setting to a definition.
    fn apply(&self, def: &mut ArgDef) {
        def.help = self.help_text();
        if let Some(c) = self.short {
            def.short = Some(c);
        }
        def.aliases = self.aliases.iter().map(|a| (*a).to_owned()).collect();
        def.short_aliases.clone_from(&self.short_aliases);
        def.hidden = self.hidden;
        def.global = self.global;
        if let Some(v) = &mut def.value {
            if let Some(name) = self.value_name {
                v.name = name.to_owned();
            }
            if self.positional {
                def.positional = true;
                def.long = None;
                v.name = self.upper_name();
            }
            #[cfg(feature = "env")]
            {
                v.env = self.env.map(str::to_owned);
            }
            v.possible = self.possible.iter().map(|p| (*p).to_owned()).collect();
        }
    }

    fn named(&self) -> Arg<bool> {
        Arg::with_id(self.field).long(self.long_name())
    }
}

impl<T> FieldSpec<T> {
    /// Start from the field name. The long name defaults to its kebab-case
    /// form.
    pub fn new(field: &'static str) -> FieldSpec<T> {
        FieldSpec {
            common: Common::new(field),
            default: None,
            default_missing: None,
        }
    }

    /// Append one doc comment line (`/// text` arrives as `" text"`).
    pub fn doc(&mut self, line: &str) {
        let line = line.strip_prefix(' ').unwrap_or(line);
        if !self.common.help.is_empty() {
            self.common.help.push('\n');
        }
        self.common.help.push_str(line);
    }
    /// Explicit help text, overriding the doc comment.
    pub fn help(&mut self, text: &'static str) {
        self.common.help_override = Some(text);
    }
    /// `long` (bare): keep the default long name.
    pub fn long_auto(&mut self) {}
    /// `long = "name"`.
    pub fn long(&mut self, name: &str) {
        self.common.long = Some(name.to_owned());
    }
    /// `short` (bare): the first character of the field name.
    pub fn short_auto(&mut self) {
        self.common.short = self.common.field.chars().next();
    }
    /// `short = 'c'`.
    pub fn short(&mut self, c: char) {
        self.common.short = Some(c);
    }
    /// `alias = "name"`.
    pub fn alias(&mut self, name: &'static str) {
        self.common.aliases.push(name);
    }
    /// `short_alias = 'c'`.
    pub fn short_alias(&mut self, c: char) {
        self.common.short_aliases.push(c);
    }
    /// `hidden`.
    pub fn hidden(&mut self) {
        self.common.hidden = true;
    }
    /// `global`.
    pub fn global(&mut self) {
        self.common.global = true;
    }
    /// `positional`.
    pub fn positional(&mut self) {
        self.common.positional = true;
    }
    /// `count` (field must be `usize`).
    pub fn count(&mut self) {
        self.common.count = true;
    }
    /// `required` (for `Vec<T>` fields: at least one value).
    pub fn required(&mut self) {
        self.common.required = true;
    }
    /// `value_name = "NAME"`.
    pub fn value_name(&mut self, name: &'static str) {
        self.common.value_name = Some(name);
    }
    /// `default = expr`.
    pub fn default(&mut self, v: T)
    where
        T: Display,
    {
        let text = v.to_string();
        self.default = Some((v, text));
    }
    /// `default_missing = expr`.
    pub fn default_missing(&mut self, v: T)
    where
        T: Display,
    {
        let text = v.to_string();
        self.default_missing = Some((v, text));
    }
    /// `env = "VAR"`.
    #[cfg(feature = "env")]
    pub fn env(&mut self, var: &'static str) {
        self.common.env = Some(var);
    }
    /// `env = "VAR"` without the feature: accepted and ignored so that the
    /// same definition compiles with and without it.
    #[cfg(not(feature = "env"))]
    pub fn env(&mut self, _var: &'static str) {}
    /// `possible = ["a", "b"]`.
    pub fn possible(&mut self, values: &'static [&'static str]) {
        self.common.possible = values;
    }

    /// Finaliser for `bool` fields.
    pub fn flag(self) -> Arg<bool> {
        let mut arg = self.common.named();
        self.common.apply(&mut arg.def);
        arg
    }
}

impl<T> FieldSpec<T>
where
    T: FromStr + Any + Clone + Send + Sync,
    T::Err: Display,
{
    fn valued(&self) -> Arg<Option<T>> {
        let mut arg = self.common.named().value::<T>();
        if let Some((v, text)) = &self.default_missing {
            arg = arg.default_missing_with(v.clone(), text.clone());
        }
        self.common.apply(&mut arg.def);
        arg
    }

    /// Finaliser for `Option<T>` fields.
    pub fn option(self) -> Arg<Option<T>> {
        self.valued()
    }

    /// Finaliser for `Vec<T>` fields.
    pub fn vec(self) -> Arg<Vec<T>> {
        let arg = self.valued().many();
        if self.common.required {
            arg.required()
        } else {
            arg
        }
    }

    /// Finaliser for plain `T` fields: required unless a default is given.
    pub fn plain(self) -> Arg<T> {
        let arg = self.valued();
        match &self.default {
            Some((v, text)) => arg.default_with(v.clone(), text.clone()),
            None => arg.required(),
        }
    }
}

impl FieldSpec<usize> {
    /// Finaliser for `usize` fields: a counter with `count`, a plain value
    /// otherwise.
    pub fn usize_field(self) -> Arg<usize> {
        if self.common.count {
            let mut arg = self.common.named().count();
            self.common.apply(&mut arg.def);
            arg
        } else {
            self.plain()
        }
    }
}

/// Settings gathered from an enum variant.
pub struct SubSpec {
    variant: &'static str,
    name: Option<&'static str>,
    about: String,
    aliases: Vec<&'static str>,
    hidden: bool,
}

impl SubSpec {
    /// Start from the variant name; the subcommand name defaults to its
    /// kebab-case form.
    pub fn new(variant: &'static str) -> SubSpec {
        SubSpec {
            variant,
            name: None,
            about: String::new(),
            aliases: Vec::new(),
            hidden: false,
        }
    }
    /// Append one doc comment line.
    pub fn doc(&mut self, line: &str) {
        let line = line.strip_prefix(' ').unwrap_or(line);
        if !self.about.is_empty() {
            self.about.push('\n');
        }
        self.about.push_str(line);
    }
    /// `name = "x"`.
    pub fn name(&mut self, name: &'static str) {
        self.name = Some(name);
    }
    /// `alias = "x"`.
    pub fn alias(&mut self, name: &'static str) {
        self.aliases.push(name);
    }
    /// `hidden`.
    pub fn hidden(&mut self) {
        self.hidden = true;
    }
    /// The subcommand name this variant answers to.
    pub fn subcommand_name(&self) -> String {
        self.name
            .map(str::to_owned)
            .unwrap_or_else(|| camel_kebab(self.variant))
    }
    /// Build the entry. `about` is the variant doc, else the payload's own
    /// about; `build` produces the payload command.
    pub fn finish(self, payload_about: Option<&'static str>, build: fn() -> Command) -> Subcommand {
        let mut sub = Subcommand::lazy(self.subcommand_name(), build);
        let about = if self.about.is_empty() {
            payload_about.map(trim_doc)
        } else {
            Some(self.about)
        };
        if let Some(a) = about {
            sub = sub.about(a);
        }
        for a in self.aliases {
            sub = sub.alias(a);
        }
        if self.hidden {
            sub = sub.hidden();
        }
        sub
    }
}

/// Command-level settings.
pub struct CommandSpec {
    name: Option<String>,
    cmd: Command,
    about: String,
}

impl CommandSpec {
    /// `default_name` is the crate name at the macro's expansion site.
    pub fn new(default_name: &str) -> CommandSpec {
        CommandSpec {
            name: None,
            cmd: Command::new(default_name),
            about: String::new(),
        }
    }
    /// Append one doc comment line.
    pub fn doc(&mut self, line: &str) {
        let line = line.strip_prefix(' ').unwrap_or(line);
        if !self.about.is_empty() {
            self.about.push('\n');
        }
        self.about.push_str(line);
    }
    /// `name = "app"`.
    pub fn name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }
    /// `version = "1.0"`.
    pub fn version(&mut self, v: impl Into<String>) {
        self.cmd.version = Some(v.into());
    }
    /// `about = "..."`, overriding the doc comment.
    pub fn about(&mut self, text: impl Into<String>) {
        self.about = text.into();
    }
    /// `long_about = "..."`.
    pub fn long_about(&mut self, text: impl Into<String>) {
        self.cmd.long_about = Some(text.into());
    }
    /// `after_help = "..."`.
    pub fn after_help(&mut self, text: impl Into<String>) {
        self.cmd.after_help = Some(text.into());
    }
    /// `disable_help`.
    pub fn disable_help(&mut self) {
        self.cmd.disable_help = true;
    }
    /// `disable_version`.
    pub fn disable_version(&mut self) {
        self.cmd.disable_version = true;
    }
    /// Produce the command, ready for `.arg()` calls.
    pub fn finish(self) -> Command {
        let mut cmd = self.cmd;
        if let Some(n) = self.name {
            cmd.name = n;
        }
        if !self.about.is_empty() {
            cmd.about = Some(self.about);
        }
        cmd
    }
}

/// Normalise a raw doc comment: strip the single leading space of each line.
pub fn trim_doc(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for (i, line) in raw.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(line.strip_prefix(' ').unwrap_or(line));
    }
    out
}

/// The command for a unit enum variant (no arguments of its own).
pub fn unit_command() -> Command {
    Command::new("")
}

/// Add every subcommand of an enum to `cmd`.
pub fn add_subcommands(mut cmd: Command, subs: Vec<Subcommand>, required: bool) -> Command {
    for s in subs {
        cmd = cmd.subcommand(s);
    }
    if required {
        cmd.subcommand_required()
    } else {
        cmd
    }
}

/// Extract a required subcommand: the parser guarantees one is present when
/// `subcommand_required()` was set, so `None` is an internal inconsistency.
pub fn required_subcommand<T>(sub: Option<T>) -> Result<T, crate::Error> {
    sub.ok_or_else(|| crate::Error::custom("a subcommand is required but one was not provided"))
}

/// Shared implementation of the generated `try_parse_from`.
pub fn parse_into<T, I>(
    cmd: &Command,
    args: I,
    from: fn(&Matches) -> Result<T, crate::Error>,
) -> Result<T, crate::Error>
where
    I: IntoIterator,
    I::Item: Into<OsString>,
{
    let m = cmd.try_parse_from(args)?;
    from(&m)
}

/// Shared implementation of the generated `try_parse_args`.
pub fn parse_args_into<T, I>(
    cmd: &Command,
    args: I,
    from: fn(&Matches) -> Result<T, crate::Error>,
) -> Result<T, crate::Error>
where
    I: IntoIterator,
    I::Item: Into<OsString>,
{
    let m = cmd.try_parse_args(args)?;
    from(&m)
}
