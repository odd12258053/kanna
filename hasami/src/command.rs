//! The command definition: [`Command`], [`Subcommand`] and [`Group`].

use std::ffi::OsString;
use std::sync::Arc;

use crate::arg::{Arg, ArgDef};
use crate::error::Error;
use crate::matches::Matches;

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
    pub(crate) hidden: bool,
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
            hidden: false,
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
            hidden: false,
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

    /// An alternative name.
    pub fn alias(mut self, name: impl Into<String>) -> Subcommand {
        self.aliases.push(name.into());
        self
    }

    /// Accept the subcommand but do not list it in help.
    pub fn hidden(mut self) -> Subcommand {
        self.hidden = true;
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
    /// Alternative names.
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }
    /// Is the subcommand hidden from help?
    pub fn is_hidden(&self) -> bool {
        self.hidden
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
        self.name == name || self.aliases.iter().any(|a| a == name)
    }
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
/// A `Command` is the intermediate representation every hasami front end
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

    /// Add an argument. The `Arg` is copied into the command; keep the
    /// original to read the value back with [`Matches::get`].
    ///
    /// Ids must be unique within a command; a duplicate is a programming
    /// error and is reported by a debug assertion.
    pub fn arg<T>(mut self, arg: &Arg<T>) -> Command {
        debug_assert!(
            !self.args.iter().any(|a| a.id == arg.def.id),
            "duplicate argument id {:?} in command {:?}",
            arg.def.id,
            self.name
        );
        debug_assert!(
            !arg.def.positional || !self.args.iter().any(|a| a.positional && a.many),
            "positional {:?} cannot follow a repeated positional in command {:?}",
            arg.def.id,
            self.name
        );
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
    /// # use hasami::{Arg, Command};
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
        let mut p = hasami_core::Parser::from_iter(args);
        crate::parse::parse(self, &mut p)
    }

    /// Parse `args`, which contain **no binary name**.
    pub fn try_parse_args<I>(&self, args: I) -> Result<Matches, Error>
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        let mut p = hasami_core::Parser::from_args(args);
        crate::parse::parse(self, &mut p)
    }

    /// Parse with an existing core [`Parser`](hasami_core::Parser). Anything
    /// the parser already consumed is not seen.
    pub fn try_parse_with(&self, parser: &mut hasami_core::Parser) -> Result<Matches, Error> {
        crate::parse::parse(self, parser)
    }

    /// The rendered help text for this command (as printed by `--help`).
    #[cfg(feature = "help")]
    pub fn render_help(&self) -> String {
        crate::help::render_help(self, &self.name)
    }

    /// The usage line for this command.
    #[cfg(feature = "help")]
    pub fn render_usage(&self) -> String {
        crate::help::render_usage(self, &self.name)
    }
}
