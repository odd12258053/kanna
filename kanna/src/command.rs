//! The command definition: [`Command`], [`Subcommand`] and [`Group`].

use std::ffi::OsString;
use std::sync::Arc;

use crate::arg::{Arg, ArgDef};
use crate::error::Error;
use crate::matches::Matches;
use crate::style::Styles;

/// A constraint over a set of arguments.
///
/// * [`required`](Group::required): at least one member must be present.
/// * [`exclusive`](Group::exclusive): at most one member may be present.
/// * both: exactly one.
///
/// "Present" means given on the command line (or through an environment
/// variable); default values do not count.
#[derive(Clone, Debug)]
pub struct Group {
    pub(crate) name: String,
    pub(crate) members: Vec<String>,
    pub(crate) required: bool,
    pub(crate) exclusive: bool,
}

impl Group {
    /// A new, empty group. The name is used in error messages and schemas.
    pub fn new(name: impl Into<String>) -> Group {
        Group {
            name: name.into(),
            members: Vec::new(),
            required: false,
            exclusive: false,
        }
    }

    /// Add an argument to the group.
    pub fn member<T>(mut self, arg: &Arg<T>) -> Group {
        self.members.push(arg.id().to_owned());
        self
    }

    /// Add an argument to the group by id.
    pub fn member_id(mut self, id: impl Into<String>) -> Group {
        self.members.push(id.into());
        self
    }

    /// At least one member must be present.
    pub fn required(mut self) -> Group {
        self.required = true;
        self
    }

    /// At most one member may be present.
    pub fn exclusive(mut self) -> Group {
        self.exclusive = true;
        self
    }

    /// The group name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The member argument ids.
    pub fn members(&self) -> &[String] {
        &self.members
    }
    /// Is at least one member required?
    pub fn is_required(&self) -> bool {
        self.required
    }
    /// Is at most one member allowed?
    pub fn is_exclusive(&self) -> bool {
        self.exclusive
    }
}

#[derive(Clone)]
pub(crate) enum Build {
    Eager(Arc<Command>),
    Lazy(Arc<dyn Fn() -> Command + Send + Sync>),
}

/// A subcommand entry in a [`Command`]: the name and summary needed for help
/// plus a way to obtain the full definition, eagerly or lazily.
#[derive(Clone)]
pub struct Subcommand {
    pub(crate) name: String,
    pub(crate) about: Option<String>,
    pub(crate) aliases: Vec<String>,
    pub(crate) visible_aliases: Vec<String>,
    pub(crate) hidden: bool,
    /// Listed in help, but not offered to AI agents as a tool.
    pub(crate) no_tool: bool,
    pub(crate) build: Build,
}

impl std::fmt::Debug for Subcommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subcommand")
            .field("name", &self.name)
            .field("about", &self.about)
            .field("lazy", &matches!(self.build, Build::Lazy(_)))
            .finish_non_exhaustive()
    }
}

impl From<Command> for Subcommand {
    fn from(cmd: Command) -> Subcommand {
        Subcommand {
            name: cmd.name.clone(),
            about: cmd.about.clone(),
            aliases: Vec::new(),
            visible_aliases: Vec::new(),
            hidden: false,
            no_tool: false,
            build: Build::Eager(Arc::new(cmd)),
        }
    }
}

impl Subcommand {
    /// A subcommand whose definition is built by `build` only when it is
    /// selected on the command line (or when a generator needs it).
    pub fn lazy<F>(name: impl Into<String>, build: F) -> Subcommand
    where
        F: Fn() -> Command + Send + Sync + 'static,
    {
        Subcommand {
            name: name.into(),
            about: None,
            aliases: Vec::new(),
            visible_aliases: Vec::new(),
            hidden: false,
            no_tool: false,
            build: Build::Lazy(Arc::new(build)),
        }
    }

    /// The one-line summary shown in the parent's help. It also becomes
    /// the command's own `about` if that was not set.
    pub fn about(mut self, text: impl Into<String>) -> Subcommand {
        let text = text.into();
        if let Build::Eager(c) = &mut self.build {
            // Unique at build time, so no clone happens.
            if let Some(c) = Arc::get_mut(c) {
                if c.about.is_none() {
                    c.about = Some(text.clone());
                }
            }
        }
        self.about = Some(text);
        self
    }

    /// An alternative name, accepted but not shown in help.
    pub fn alias(mut self, name: impl Into<String>) -> Subcommand {
        self.aliases.push(name.into());
        self
    }

    /// An alternative name shown in help as `[aliases: name]`.
    pub fn visible_alias(mut self, name: impl Into<String>) -> Subcommand {
        self.visible_aliases.push(name.into());
        self
    }

    /// Accept the subcommand but do not list it in help.
    pub fn hidden(mut self) -> Subcommand {
        self.hidden = true;
        self
    }

    /// Keep the subcommand in help and completion, but leave it (and its
    /// own subcommands) out of the tool definitions that `kanna-schema`
    /// derives for AI agents. For commands meant for people or for the
    /// driver itself: printing the tool list, running a tool from JSON,
    /// interactive setup.
    pub fn no_tool(mut self) -> Subcommand {
        self.no_tool = true;
        self
    }

    /// The subcommand name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The summary, if any.
    pub fn summary(&self) -> Option<&str> {
        self.about.as_deref()
    }
    /// Alternative names that are not shown in help.
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }
    /// Alternative names that are shown in help.
    pub fn visible_aliases(&self) -> &[String] {
        &self.visible_aliases
    }
    /// The name and every alias.
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.name.as_str())
            .chain(self.aliases.iter().map(String::as_str))
            .chain(self.visible_aliases.iter().map(String::as_str))
    }
    /// Is the subcommand hidden from help?
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Is the subcommand kept out of agent tool definitions
    /// ([`no_tool`](Subcommand::no_tool))?
    pub fn is_no_tool(&self) -> bool {
        self.no_tool
    }

    /// Build (or clone) the full definition. The entry's name and summary
    /// take precedence over what a lazily built command says about itself.
    pub fn build(&self) -> Command {
        match &self.build {
            Build::Eager(c) => (**c).clone(),
            Build::Lazy(f) => self.adopt(f()),
        }
    }

    /// Make a lazily built `cmd` agree with this entry: the entry's name is
    /// authoritative (`Subcommand::lazy("add", ..)` may build a command with
    /// another internal name) and the entry's summary fills in a missing
    /// `about`.
    fn adopt(&self, mut cmd: Command) -> Command {
        cmd.name.clone_from(&self.name);
        if cmd.about.is_none() {
            cmd.about.clone_from(&self.about);
        }
        cmd
    }

    pub(crate) fn resolve(&self) -> CmdRef<'static> {
        match &self.build {
            Build::Eager(c) => CmdRef::Shared(Arc::clone(c)),
            Build::Lazy(f) => CmdRef::Owned(Box::new(self.adopt(f()))),
        }
    }

    pub(crate) fn matches_name(&self, name: &str) -> bool {
        self.names().any(|n| n == name)
    }
}

/// Split a command line into words the way a POSIX shell does for the
/// simple cases: whitespace separates, single quotes take everything
/// literally, double quotes allow `\"` and `\\`.
pub(crate) fn shell_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            '\'' => {
                in_word = true;
                for c in chars.by_ref() {
                    if c == '\'' {
                        break;
                    }
                    cur.push(c);
                }
            }
            '"' => {
                in_word = true;
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => {
                            if let Some(n) = chars.next() {
                                cur.push(n);
                            }
                        }
                        c => cur.push(c),
                    }
                }
            }
            '\\' => {
                in_word = true;
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            c => {
                in_word = true;
                cur.push(c);
            }
        }
    }
    if in_word {
        words.push(cur);
    }
    words
}

/// A borrowed, shared or freshly built command.
pub(crate) enum CmdRef<'a> {
    Borrowed(&'a Command),
    Shared(Arc<Command>),
    Owned(Box<Command>),
}

impl std::ops::Deref for CmdRef<'_> {
    type Target = Command;
    fn deref(&self) -> &Command {
        match self {
            CmdRef::Borrowed(c) => c,
            CmdRef::Shared(c) => c,
            CmdRef::Owned(c) => c,
        }
    }
}

/// The definition of a command line interface: arguments, subcommands,
/// constraints and metadata.
///
/// A `Command` is the intermediate representation every kanna front end
/// lowers to: the builder API constructs it directly, the [`cli!`](crate::cli)
/// macro and `#[derive(Args)]` generate code that constructs it, and the
/// generators (`complete`, `doc`, `schema`) read it.
///
/// Parsing never mutates the command, so one definition can be reused.
#[derive(Clone, Debug)]
pub struct Command {
    pub(crate) name: String,
    pub(crate) version: Option<String>,
    pub(crate) about: Option<String>,
    pub(crate) long_about: Option<String>,
    pub(crate) after_help: Option<String>,
    pub(crate) args: Vec<ArgDef>,
    pub(crate) subcommands: Vec<Subcommand>,
    pub(crate) groups: Vec<Group>,
    /// `(a, b)`: if `a` is present, `b` must be too.
    pub(crate) requires: Vec<(String, String)>,
    pub(crate) subcommand_required: bool,
    pub(crate) disable_help: bool,
    pub(crate) disable_version: bool,
    pub(crate) before_help: Option<String>,
    pub(crate) long_version: Option<String>,
    pub(crate) args_override_self: bool,
    pub(crate) arg_required_else_help: bool,
    pub(crate) infer_long_args: bool,
    pub(crate) infer_subcommands: bool,
    pub(crate) external_subcommands: bool,
    /// Wrap help at this many columns; `None` means the `COLUMNS`
    /// environment variable or 100.
    pub(crate) term_width: Option<usize>,
    pub(crate) styles: Styles,
    /// Complete command lines shown under `Examples:` in help and
    /// checked by [`check_examples`](Command::check_examples).
    pub(crate) examples: Vec<String>,
}

impl Command {
    /// A new command. `name` is used in usage lines and as the subcommand
    /// name when nested.
    pub fn new(name: impl Into<String>) -> Command {
        Command {
            name: name.into(),
            version: None,
            about: None,
            long_about: None,
            after_help: None,
            args: Vec::new(),
            subcommands: Vec::new(),
            groups: Vec::new(),
            requires: Vec::new(),
            subcommand_required: false,
            disable_help: false,
            disable_version: false,
            before_help: None,
            long_version: None,
            args_override_self: false,
            arg_required_else_help: false,
            infer_long_args: false,
            infer_subcommands: false,
            external_subcommands: false,
            term_width: None,
            styles: Styles::default_palette(),
            examples: Vec::new(),
        }
    }

    /// The version printed by `-V/--version`. Setting it enables those flags.
    pub fn version(mut self, v: impl Into<String>) -> Command {
        self.version = Some(v.into());
        self
    }

    /// One-line description shown at the top of help and in the parent's
    /// subcommand list.
    pub fn about(mut self, text: impl Into<String>) -> Command {
        self.about = Some(text.into());
        self
    }

    /// Longer description shown in this command's help instead of `about`.
    pub fn long_about(mut self, text: impl Into<String>) -> Command {
        self.long_about = Some(text.into());
        self
    }

    /// Free text appended to the end of help.
    pub fn after_help(mut self, text: impl Into<String>) -> Command {
        self.after_help = Some(text.into());
        self
    }

    /// Add an example command line. Examples are listed under
    /// `Examples:` in help (after the options, before `after_help`) and
    /// verified by [`check_examples`](Command::check_examples). Write the
    /// full line as a user would type it, starting with the command name:
    /// `"app add cake --sweet"`. A `#` starts a comment that is shown but
    /// not parsed.
    pub fn example(mut self, line: impl Into<String>) -> Command {
        self.examples.push(line.into());
        self
    }

    /// Free text shown at the very top of help, before the description.
    pub fn before_help(mut self, text: impl Into<String>) -> Command {
        self.before_help = Some(text.into());
        self
    }

    /// A longer version text printed by `--version`; `-V` keeps printing
    /// [`version`](Command::version).
    pub fn long_version(mut self, text: impl Into<String>) -> Command {
        self.long_version = Some(text.into());
        self
    }

    /// Let every argument of this command be repeated, the last value
    /// winning, instead of reporting an error. Per-argument opt-in is
    /// [`Arg::last_wins`].
    pub fn args_override_self(mut self) -> Command {
        self.args_override_self = true;
        self
    }

    /// When the command is invoked with no arguments at all, print help to
    /// stderr and exit with status 2
    /// ([`ErrorKind::HelpOnMissingArgs`](crate::ErrorKind::HelpOnMissingArgs)).
    pub fn arg_required_else_help(mut self) -> Command {
        self.arg_required_else_help = true;
        self
    }

    /// Accept an unambiguous prefix of a long option name (`--verb` for
    /// `--verbose`). Exact matches always win.
    pub fn infer_long_args(mut self) -> Command {
        self.infer_long_args = true;
        self
    }

    /// Accept an unambiguous prefix of a subcommand name (`inst` for
    /// `install`).
    pub fn infer_subcommands(mut self) -> Command {
        self.infer_subcommands = true;
        self
    }

    /// Treat a first positional that is not a known subcommand as an
    /// external subcommand: it and everything after it are handed back
    /// untouched through
    /// [`Matches::external_subcommand`](crate::Matches::external_subcommand).
    pub fn allow_external_subcommands(mut self) -> Command {
        self.external_subcommands = true;
        self
    }

    /// Wrap help text at `columns`. The default is the `COLUMNS`
    /// environment variable when set, otherwise 100; `0` disables wrapping.
    pub fn term_width(mut self, columns: usize) -> Command {
        self.term_width = Some(columns);
        self
    }

    /// The colour palette used for help and errors when colour is on
    /// (feature `color`). Without the feature the palette has no effect.
    pub fn styles(mut self, styles: Styles) -> Command {
        self.styles = styles;
        self
    }

    /// Add an argument. The `Arg` is copied into the command; keep the
    /// original to read the value back with [`Matches::get`].
    ///
    /// Ids must be unique within a command. That and every other rule
    /// about a well-formed definition is checked by
    /// [`validate`](Command::validate), which parsing runs in debug builds.
    pub fn arg<T>(mut self, arg: &Arg<T>) -> Command {
        self.args.push(arg.def.clone());
        self
    }

    /// Add a subcommand, eagerly (from a `Command`) or lazily (from a
    /// [`Subcommand::lazy`]).
    pub fn subcommand(mut self, sub: impl Into<Subcommand>) -> Command {
        self.subcommands.push(sub.into());
        self
    }

    /// Add a subcommand that is built only when selected.
    pub fn subcommand_lazy<F>(
        self,
        name: impl Into<String>,
        about: impl Into<String>,
        build: F,
    ) -> Command
    where
        F: Fn() -> Command + Send + Sync + 'static,
    {
        self.subcommand(Subcommand::lazy(name, build).about(about))
    }

    /// Add a constraint group.
    pub fn group(mut self, group: Group) -> Command {
        self.groups.push(group);
        self
    }

    /// At most one of the given argument ids may be present.
    ///
    /// ```
    /// # use kanna::{Arg, Command};
    /// let json = Arg::new("json");
    /// let yaml = Arg::new("yaml");
    /// let cmd = Command::new("x").arg(&json).arg(&yaml).exclusive([json.id(), yaml.id()]);
    /// assert!(cmd.try_parse_args(["--json", "--yaml"]).is_err());
    /// ```
    pub fn exclusive<'a>(self, ids: impl IntoIterator<Item = &'a str>) -> Command {
        let mut g = Group::new("").exclusive();
        for id in ids {
            g = g.member_id(id);
        }
        self.group(g)
    }

    /// If `arg` is present, `needs` must be present too.
    pub fn requires<A, B>(mut self, arg: &Arg<A>, needs: &Arg<B>) -> Command {
        self.requires
            .push((arg.id().to_owned(), needs.id().to_owned()));
        self
    }

    /// Fail when no subcommand is given.
    pub fn subcommand_required(mut self) -> Command {
        self.subcommand_required = true;
        self
    }

    /// Do not add `-h/--help`.
    pub fn disable_help(mut self) -> Command {
        self.disable_help = true;
        self
    }

    /// Do not add `-V/--version` even when a version is set.
    pub fn disable_version(mut self) -> Command {
        self.disable_version = true;
        self
    }

    // ------------------------------------------------------------ getters

    /// The command name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The version string.
    pub fn get_version(&self) -> Option<&str> {
        self.version.as_deref()
    }
    /// The one-line description.
    pub fn get_about(&self) -> Option<&str> {
        self.about.as_deref()
    }
    /// The long description.
    pub fn get_long_about(&self) -> Option<&str> {
        self.long_about.as_deref()
    }
    /// The text appended to help.
    pub fn get_after_help(&self) -> Option<&str> {
        self.after_help.as_deref()
    }
    /// The argument definitions, in the order they were added.
    pub fn args(&self) -> &[ArgDef] {
        &self.args
    }
    /// The subcommand entries.
    pub fn subcommands(&self) -> &[Subcommand] {
        &self.subcommands
    }
    /// The constraint groups.
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }
    /// The `requires` relations as `(arg id, required id)` pairs.
    pub fn requirements(&self) -> &[(String, String)] {
        &self.requires
    }
    /// Must a subcommand be given?
    pub fn is_subcommand_required(&self) -> bool {
        self.subcommand_required
    }
    /// Does the command accept `-h/--help`?
    pub fn has_help_flag(&self) -> bool {
        cfg!(feature = "help") && !self.disable_help
    }
    /// The example command lines.
    pub fn get_examples(&self) -> &[String] {
        &self.examples
    }
    /// The text printed by `--version` when it differs from `-V`.
    pub fn get_long_version(&self) -> Option<&str> {
        self.long_version.as_deref()
    }
    /// The text shown before the description in help.
    pub fn get_before_help(&self) -> Option<&str> {
        self.before_help.as_deref()
    }
    /// May every argument be repeated with the last value winning?
    pub fn is_args_override_self(&self) -> bool {
        self.args_override_self
    }
    /// Is help shown when no arguments are given?
    pub fn is_arg_required_else_help(&self) -> bool {
        self.arg_required_else_help
    }
    /// Are unambiguous prefixes of long option names accepted?
    pub fn is_infer_long_args(&self) -> bool {
        self.infer_long_args
    }
    /// Are unambiguous prefixes of subcommand names accepted?
    pub fn is_infer_subcommands(&self) -> bool {
        self.infer_subcommands
    }
    /// Are unknown subcommands passed through?
    pub fn allows_external_subcommands(&self) -> bool {
        self.external_subcommands
    }
    /// The configured help width, if one was set explicitly.
    pub fn get_term_width(&self) -> Option<usize> {
        self.term_width
    }
    /// The colour palette.
    pub fn get_styles(&self) -> &Styles {
        &self.styles
    }
    /// Does the command accept `-V/--version`?
    pub fn has_version_flag(&self) -> bool {
        cfg!(feature = "help") && !self.disable_version && self.version.is_some()
    }

    /// Look up an argument by id.
    pub fn find_arg(&self, id: &str) -> Option<&ArgDef> {
        self.args.iter().find(|a| a.id == id)
    }

    /// Look up a subcommand by name or alias.
    pub fn find_subcommand(&self, name: &str) -> Option<&Subcommand> {
        self.subcommands.iter().find(|s| s.matches_name(name))
    }

    // --------------------------------------------------------- validation

    /// Check that the definition is well formed: unique ids and names,
    /// positionals in a valid order, constraints that reference existing
    /// arguments, defaults that are among the possible values, and the
    /// same for every subcommand (lazy ones are built).
    ///
    /// Every problem found is returned, as `command: description`. The
    /// parse entry points call this in debug builds and panic on a
    /// problem, so a mistake shows up in the first test run; release builds
    /// skip the check.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut problems = Vec::new();
        self.validate_into(&self.name, &mut problems);
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
        }
    }

    fn validate_into(&self, path: &str, out: &mut Vec<String>) {
        let mut report = |what: String| out.push([path, ": ", &what].concat());
        let mut ids: Vec<&str> = Vec::new();
        // `-h/--help` and `-V/--version` are not reserved: a user-defined
        // argument with one of those names replaces the synthetic flag.
        let mut longs: Vec<&str> = Vec::new();
        let mut shorts: Vec<char> = Vec::new();
        let mut seen_optional_positional = false;
        let mut seen_many_positional = false;
        for a in &self.args {
            let id = a.id.as_str();
            if id.is_empty() {
                report("an argument has an empty id".to_owned());
            }
            if ids.contains(&id) {
                report(format!("duplicate argument id '{id}'"));
                continue;
            }
            ids.push(id);
            if a.positional {
                if a.long.is_some() || a.short.is_some() {
                    report(format!(
                        "positional '{id}' cannot have a long or short name"
                    ));
                }
                if a.global {
                    report(format!("positional '{id}' cannot be global"));
                }
                if a.value
                    .as_ref()
                    .is_some_and(|v| v.default_missing.is_some())
                {
                    report(format!("positional '{id}' cannot have an optional value"));
                }
                if seen_many_positional {
                    report(format!("positional '{id}' follows a repeated positional"));
                }
                if a.required && seen_optional_positional {
                    report(format!(
                        "required positional '{id}' follows an optional one"
                    ));
                }
                if a.trailing && !a.many {
                    report(format!(
                        "trailing positional '{id}' must be repeated (many)"
                    ));
                }
                seen_many_positional |= a.many;
                seen_optional_positional |= !a.required;
            } else {
                if a.long.is_none() && a.short.is_none() {
                    report(format!("option '{id}' has neither a long nor a short name"));
                }
                for l in a.long_names() {
                    if l.is_empty() || l.starts_with('-') || l.contains(['=', ' ']) {
                        report(format!("option '{id}' has an invalid long name '{l}'"));
                    }
                    if longs.contains(&l) {
                        report(format!("long name '--{l}' is used more than once"));
                    }
                    longs.push(l);
                }
                for c in a.short.iter().chain(&a.short_aliases) {
                    if *c == '-' || c.is_whitespace() {
                        report(format!("option '{id}' has an invalid short name '{c}'"));
                    }
                    if shorts.contains(c) {
                        report(format!("short name '-{c}' is used more than once"));
                    }
                    shorts.push(*c);
                }
                if a.trailing || (a.greedy && !a.many) {
                    report(format!(
                        "option '{id}' has a positional-only or many-only setting"
                    ));
                }
            }
            if a.count && a.value.is_some() {
                report(format!("'{id}' cannot both count and take a value"));
            }
            if let Some(v) = &a.value {
                if !v.possible.is_empty() {
                    if let Some((_, d)) = &v.default {
                        if !v.possible.contains(d) {
                            report(format!("default '{d}' of '{id}' is not a possible value"));
                        }
                    }
                }
                if v.delimiter.is_some() && !a.many {
                    report(format!("'{id}' has a delimiter but is not repeated (many)"));
                }
            }
        }
        // References between arguments.
        let known = |id: &str| self.args.iter().any(|a| a.id == id);
        for a in &self.args {
            for r in &a.relations {
                let other = r.other();
                if !known(other) {
                    report(format!("'{}' refers to unknown argument '{other}'", a.id));
                }
            }
        }
        for (a, b) in &self.requires {
            for id in [a, b] {
                if !known(id) {
                    report(format!("requires refers to unknown argument '{id}'"));
                }
            }
        }
        for g in &self.groups {
            for id in &g.members {
                if !known(id) {
                    report(format!(
                        "group '{}' refers to unknown argument '{id}'",
                        g.name
                    ));
                }
            }
            if g.exclusive && g.members.len() < 2 {
                report(format!(
                    "exclusive group '{}' has fewer than two members",
                    g.name
                ));
            }
        }
        // Subcommands.
        let mut names: Vec<&str> = Vec::new();
        for s in &self.subcommands {
            for n in s.names() {
                if n.is_empty() {
                    report("a subcommand has an empty name".to_owned());
                }
                if names.contains(&n) {
                    report(format!("subcommand name '{n}' is used more than once"));
                }
                names.push(n);
            }
        }
        if self.subcommand_required && self.subcommands.is_empty() {
            report("subcommand_required is set but there are no subcommands".to_owned());
        }
        if self.args.iter().any(|a| a.positional && a.required) && !self.subcommands.is_empty() {
            report("a required positional cannot be combined with subcommands".to_owned());
        }
        for s in &self.subcommands {
            let sub = s.build();
            sub.validate_into(&[path, " ", &s.name].concat(), out);
        }
    }

    /// Parse every example (of this command and, recursively, of its
    /// subcommands) and return the ones that fail, as
    /// `` `example`: error message ``. Help and version requests count as
    /// success.
    ///
    /// An example is split like a shell would (spaces separate words,
    /// single or double quotes group them, `\` escapes inside double
    /// quotes) and the text after `#` is ignored. Words up to and
    /// including the command's own name are dropped, so an example may
    /// start with `app` or `app sub`.
    ///
    /// ```
    /// use kanna::{Arg, Command};
    /// let n = Arg::new("number").value::<u32>();
    /// let cmd = Command::new("app").arg(&n)
    ///     .example("app --number 3")
    ///     .example("app --number many   # wrong on purpose");
    /// let failed = cmd.check_examples().unwrap_err();
    /// assert_eq!(failed.len(), 1);
    /// assert!(failed[0].starts_with("`app --number many`: "));
    /// ```
    pub fn check_examples(&self) -> Result<(), Vec<String>> {
        let mut failed = Vec::new();
        self.check_examples_into(&mut failed);
        if failed.is_empty() {
            Ok(())
        } else {
            Err(failed)
        }
    }

    fn check_examples_into(&self, out: &mut Vec<String>) {
        for ex in &self.examples {
            let line = ex.split('#').next().unwrap_or("").trim();
            let mut words = shell_words(line);
            if let Some(i) = words.iter().position(|w| w == &self.name) {
                words.drain(..=i);
            }
            if let Err(e) = self.try_parse_args(words) {
                if !e.is_display() {
                    out.push(["`", line, "`: ", e.message()].concat());
                }
            }
        }
        for s in &self.subcommands {
            s.build().check_examples_into(out);
        }
    }

    // ------------------------------------------------------------ parsing

    /// Parse the process arguments. On error, print the message (help and
    /// version go to stdout, errors to stderr) and exit with the appropriate
    /// status (0 for help/version, 2 otherwise).
    pub fn parse(&self) -> Matches {
        self.try_parse().unwrap_or_else(|e| e.exit())
    }

    /// Parse the process arguments.
    pub fn try_parse(&self) -> Result<Matches, Error> {
        self.try_parse_from(std::env::args_os())
    }

    /// Parse `args`, whose **first item is the binary name** (like
    /// `std::env::args_os()`), exiting on error like [`parse`](Command::parse).
    pub fn parse_from<I>(&self, args: I) -> Matches
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        self.try_parse_from(args).unwrap_or_else(|e| e.exit())
    }

    /// Parse `args`, whose **first item is the binary name**.
    pub fn try_parse_from<I>(&self, args: I) -> Result<Matches, Error>
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        let mut p = kanna_core::Parser::from_iter(args);
        crate::parse::parse(self, &mut p)
    }

    /// Parse `args`, which contain **no binary name**.
    pub fn try_parse_args<I>(&self, args: I) -> Result<Matches, Error>
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        let mut p = kanna_core::Parser::from_args(args);
        crate::parse::parse(self, &mut p)
    }

    /// Parse with an existing core [`Parser`](kanna_core::Parser). Anything
    /// the parser already consumed is not seen.
    pub fn try_parse_with(&self, parser: &mut kanna_core::Parser) -> Result<Matches, Error> {
        crate::parse::parse(self, parser)
    }

    /// The rendered help text for this command as printed by `--help`
    /// (long descriptions where they exist).
    #[cfg(feature = "help")]
    pub fn render_help(&self) -> String {
        crate::help::render_help(self, &self.name, true)
    }

    /// The rendered help text as printed by `-h` (short descriptions).
    #[cfg(feature = "help")]
    pub fn render_short_help(&self) -> String {
        crate::help::render_help(self, &self.name, false)
    }

    /// The usage line for this command.
    #[cfg(feature = "help")]
    pub fn render_usage(&self) -> String {
        crate::help::render_usage(self, &self.name)
    }
}
