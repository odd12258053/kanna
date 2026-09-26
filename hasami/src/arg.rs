//! Argument definitions: the type-state builder [`Arg`] and the untyped
//! definition [`ArgDef`] it produces.

use std::any::Any;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

use crate::matches::Stored;

/// A parsed value, stored type-erased in [`Matches`](crate::Matches).
pub(crate) type AnyValue = Box<dyn Any + Send + Sync>;

/// Why a value could not be turned into its typed form.
pub(crate) enum ParseFailure {
    /// The raw value was not valid Unicode and the parser needed a `&str`.
    NonUnicode,
    /// The parser rejected the value; the string is the reason.
    Invalid(String),
}

pub(crate) type ValueParser = Arc<dyn Fn(&OsStr) -> Result<AnyValue, ParseFailure> + Send + Sync>;
pub(crate) type ValueMaker = Arc<dyn Fn() -> AnyValue + Send + Sync>;
/// Produces completion candidates for a partially typed value.
pub type Completer = Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>;

/// A function that turns the stored occurrences of an argument into the
/// typed value the user asked for. `None` means "the stored values do not
/// have the expected type", which only happens when a key is used with the
/// wrong [`Matches`](crate::Matches).
pub(crate) type Extract<T> = fn(&ArgDef, &[Stored]) -> Option<T>;

/// How a value-taking argument behaves.
#[derive(Clone)]
pub(crate) struct ValueDef {
    pub(crate) name: String,
    pub(crate) parser: ValueParser,
    /// Value used when the argument is absent, plus its help rendering.
    pub(crate) default: Option<(ValueMaker, String)>,
    /// Value used when the option is given without a value (`--color`),
    /// plus its help rendering. Makes the value optional.
    pub(crate) default_missing: Option<(ValueMaker, String)>,
    pub(crate) possible: Vec<String>,
    #[cfg(feature = "env")]
    pub(crate) env: Option<String>,
    pub(crate) completer: Option<Completer>,
}

/// The untyped definition of one argument, as stored in a
/// [`Command`](crate::Command).
///
/// Obtained through [`Command::args`](crate::Command::args). Only getters are
/// public; construct arguments with [`Arg`].
#[derive(Clone)]
pub struct ArgDef {
    pub(crate) id: String,
    pub(crate) long: Option<String>,
    pub(crate) short: Option<char>,
    pub(crate) aliases: Vec<String>,
    pub(crate) short_aliases: Vec<char>,
    pub(crate) positional: bool,
    pub(crate) value: Option<ValueDef>,
    pub(crate) help: Option<String>,
    pub(crate) hidden: bool,
    pub(crate) global: bool,
    pub(crate) required: bool,
    pub(crate) many: bool,
    pub(crate) count: bool,
}

impl std::fmt::Debug for ArgDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArgDef")
            .field("id", &self.id)
            .field("long", &self.long)
            .field("short", &self.short)
            .field("positional", &self.positional)
            .field("takes_value", &self.value.is_some())
            .field("required", &self.required)
            .field("many", &self.many)
            .finish_non_exhaustive()
    }
}

impl ArgDef {
    fn new(id: String) -> ArgDef {
        ArgDef {
            id,
            long: None,
            short: None,
            aliases: Vec::new(),
            short_aliases: Vec::new(),
            positional: false,
            value: None,
            help: None,
            hidden: false,
            global: false,
            required: false,
            many: false,
            count: false,
        }
    }

    /// The identifier used to look the argument up in [`Matches`](crate::Matches)
    /// and to reference it from groups.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// The long name (`--name`), without dashes.
    pub fn long(&self) -> Option<&str> {
        self.long.as_deref()
    }
    /// The short name (`-n`).
    pub fn short(&self) -> Option<char> {
        self.short
    }
    /// Additional long names.
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }
    /// Additional short names.
    pub fn short_aliases(&self) -> &[char] {
        &self.short_aliases
    }
    /// Is this a positional argument?
    pub fn is_positional(&self) -> bool {
        self.positional
    }
    /// Does the argument take a value? (`false` for flags and counters.)
    pub fn takes_value(&self) -> bool {
        self.value.is_some()
    }
    /// Is the value optional (`--color[=WHEN]`)?
    pub fn value_is_optional(&self) -> bool {
        self.value
            .as_ref()
            .is_some_and(|v| v.default_missing.is_some())
    }
    /// The placeholder shown for the value in help (`<NAME>`).
    pub fn value_name(&self) -> Option<&str> {
        self.value.as_ref().map(|v| v.name.as_str())
    }
    /// The help text.
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }
    /// Is the argument hidden from help?
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }
    /// Is the argument accepted by all subcommands below the one defining it?
    pub fn is_global(&self) -> bool {
        self.global
    }
    /// Must the argument be present?
    pub fn is_required(&self) -> bool {
        self.required
    }
    /// May the argument be repeated, collecting every value?
    pub fn is_many(&self) -> bool {
        self.many
    }
    /// Is this a counting flag (`-vvv`)?
    pub fn is_count(&self) -> bool {
        self.count
    }
    /// The default value as shown in help.
    pub fn default_text(&self) -> Option<&str> {
        self.value
            .as_ref()
            .and_then(|v| v.default.as_ref())
            .map(|d| d.1.as_str())
    }
    /// The value used when the option is given without one, as shown in help.
    pub fn default_missing_text(&self) -> Option<&str> {
        self.value
            .as_ref()
            .and_then(|v| v.default_missing.as_ref())
            .map(|d| d.1.as_str())
    }
    /// The accepted values, if restricted.
    pub fn possible_values(&self) -> &[String] {
        self.value.as_ref().map_or(&[], |v| v.possible.as_slice())
    }
    /// The dynamic completion function, if one was set with
    /// [`Arg::complete_with`].
    pub fn completer(&self) -> Option<&Completer> {
        self.value.as_ref().and_then(|v| v.completer.as_ref())
    }
    /// The environment variable consulted when the argument is absent.
    #[cfg(feature = "env")]
    pub fn env(&self) -> Option<&str> {
        self.value.as_ref().and_then(|v| v.env.as_deref())
    }

    /// How the argument is written on the command line, for messages:
    /// `--name <NAME>`, `-v`, `<INPUT>`.
    pub fn display_name(&self) -> String {
        let mut s = String::new();
        if self.positional {
            s.push('<');
            s.push_str(self.value_name().unwrap_or(&self.id));
            s.push('>');
            return s;
        }
        match (self.short, &self.long) {
            (_, Some(long)) => {
                s.push_str("--");
                s.push_str(long);
            }
            (Some(c), None) => {
                s.push('-');
                s.push(c);
            }
            (None, None) => s.push_str(&self.id),
        }
        if let Some(v) = &self.value {
            if v.default_missing.is_some() {
                s.push_str("[=<");
                s.push_str(&v.name);
                s.push_str(">]");
            } else {
                s.push_str(" <");
                s.push_str(&v.name);
                s.push('>');
            }
        }
        s
    }

    /// The names an option can be spelled with (`-n`, `--name`, aliases).
    pub fn spellings(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(c) = self.short {
            out.push(format!("-{c}"));
        }
        if let Some(l) = &self.long {
            out.push(format!("--{l}"));
        }
        out.extend(self.short_aliases.iter().map(|c| format!("-{c}")));
        out.extend(self.aliases.iter().map(|l| format!("--{l}")));
        out
    }

    pub(crate) fn matches_long(&self, name: &str) -> bool {
        self.long.as_deref() == Some(name) || self.aliases.iter().any(|a| a == name)
    }

    pub(crate) fn matches_short(&self, c: char) -> bool {
        self.short == Some(c) || self.short_aliases.contains(&c)
    }
}

fn value_name_from_id(id: &str) -> String {
    id.to_ascii_uppercase().replace('-', "_")
}

/// A typed argument definition and, after parsing, the key used to read its
/// value back out of [`Matches`](crate::Matches).
///
/// `T` is the type [`Matches::get`](crate::Matches::get) returns. The builder
/// moves between types:
///
/// | Start                 | Method                     | Result          |
/// |-----------------------|----------------------------|-----------------|
/// | `Arg::new("x")`       |                            | `Arg<bool>`     |
/// | `Arg<bool>`           | `.count()`                 | `Arg<usize>`    |
/// | `Arg<bool>`           | `.value::<T>()`            | `Arg<Option<T>>`|
/// | `Arg::positional::<T>("X")` |                      | `Arg<Option<T>>`|
/// | `Arg<Option<T>>`      | `.required()`, `.default(v)` | `Arg<T>`      |
/// | `Arg<Option<T>>`      | `.many()`                  | `Arg<Vec<T>>`   |
///
/// ```
/// use hasami::{Arg, Command};
///
/// let verbose = Arg::new("verbose").short('v').help("Say more");
/// let name = Arg::new("name").short('n').value::<String>().required();
/// let count = Arg::new("count").value::<u32>().default(1);
/// let cmd = Command::new("greet").arg(&verbose).arg(&name).arg(&count);
///
/// let m = cmd.try_parse_args(["-v", "--name", "bob"]).unwrap();
/// assert!(m.get(&verbose));
/// assert_eq!(m.get(&name), "bob");
/// assert_eq!(m.get(&count), 1);
/// ```
pub struct Arg<T> {
    pub(crate) def: ArgDef,
    pub(crate) extract: Extract<T>,
    _t: PhantomData<fn() -> T>,
}

impl<T> Clone for Arg<T> {
    fn clone(&self) -> Self {
        Arg {
            def: self.def.clone(),
            extract: self.extract,
            _t: PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for Arg<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.def.fmt(f)
    }
}

// ------------------------------------------------------------ extractors

fn ex_flag(_: &ArgDef, v: &[Stored]) -> Option<bool> {
    Some(!v.is_empty())
}

fn ex_count(_: &ArgDef, v: &[Stored]) -> Option<usize> {
    Some(v.len())
}

fn ex_option<U: Any + Clone>(_: &ArgDef, v: &[Stored]) -> Option<Option<U>> {
    match v.last() {
        None => Some(None),
        Some(s) => s.value.downcast_ref::<U>().cloned().map(Some),
    }
}

fn ex_required<U: Any + Clone>(_: &ArgDef, v: &[Stored]) -> Option<U> {
    v.last().and_then(|s| s.value.downcast_ref::<U>().cloned())
}

fn ex_vec<U: Any + Clone>(_: &ArgDef, v: &[Stored]) -> Option<Vec<U>> {
    v.iter()
        .map(|s| s.value.downcast_ref::<U>().cloned())
        .collect()
}

fn maker<U: Any + Clone + Send + Sync>(v: U) -> ValueMaker {
    Arc::new(move || Box::new(v.clone()))
}

fn str_parser<U, F, E>(f: F) -> ValueParser
where
    U: Any + Send + Sync,
    F: Fn(&str) -> Result<U, E> + Send + Sync + 'static,
    E: Display,
{
    Arc::new(move |raw: &OsStr| {
        let s = raw.to_str().ok_or(ParseFailure::NonUnicode)?;
        f(s).map(|v| Box::new(v) as AnyValue)
            .map_err(|e| ParseFailure::Invalid(e.to_string()))
    })
}

fn os_parser<U, F, E>(f: F) -> ValueParser
where
    U: Any + Send + Sync,
    F: Fn(&OsStr) -> Result<U, E> + Send + Sync + 'static,
    E: Display,
{
    Arc::new(move |raw: &OsStr| {
        f(raw)
            .map(|v| Box::new(v) as AnyValue)
            .map_err(|e| ParseFailure::Invalid(e.to_string()))
    })
}

impl<T> Arg<T> {
    fn with(def: ArgDef, extract: Extract<T>) -> Arg<T> {
        Arg {
            def,
            extract,
            _t: PhantomData,
        }
    }

    fn retype<U>(self, extract: Extract<U>) -> Arg<U> {
        Arg::with(self.def, extract)
    }

    /// The identifier of this argument (see [`ArgDef::id`]).
    pub fn id(&self) -> &str {
        &self.def.id
    }

    /// The untyped definition.
    pub fn def(&self) -> &ArgDef {
        &self.def
    }

    /// Set (or replace) the long name.
    pub fn long(mut self, name: impl Into<String>) -> Self {
        self.def.long = Some(name.into());
        self
    }

    /// Set (or replace) the short name.
    pub fn short(mut self, c: char) -> Self {
        self.def.short = Some(c);
        self
    }

    /// Add an alternative long name. Aliases are accepted but not shown in help.
    pub fn alias(mut self, name: impl Into<String>) -> Self {
        self.def.aliases.push(name.into());
        self
    }

    /// Add an alternative short name.
    pub fn short_alias(mut self, c: char) -> Self {
        self.def.short_aliases.push(c);
        self
    }

    /// The help text shown next to the argument.
    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.def.help = Some(text.into());
        self
    }

    /// Accept the argument but do not list it in help.
    pub fn hidden(mut self) -> Self {
        self.def.hidden = true;
        self
    }

    /// Make the argument available to every subcommand below the command
    /// that defines it. Its value is stored in the defining command's
    /// [`Matches`](crate::Matches).
    pub fn global(mut self) -> Self {
        self.def.global = true;
        self
    }

    /// The placeholder for the value in help and usage (`<NAME>`).
    pub fn value_name(mut self, name: impl Into<String>) -> Self {
        if let Some(v) = &mut self.def.value {
            v.name = name.into();
        }
        self
    }
}

impl Arg<bool> {
    fn flag(id: String) -> Arg<bool> {
        Arg::with(ArgDef::new(id), ex_flag)
    }

    /// A flag whose id and long name are both `name`: `Arg::new("verbose")`
    /// is `--verbose`. Add a short name with [`short`](Arg::short); turn it
    /// into an option with [`value`](Arg::value).
    pub fn new(name: impl Into<String>) -> Arg<bool> {
        let name = name.into();
        let mut a = Arg::flag(name.clone());
        a.def.long = Some(name);
        a
    }

    /// A flag with an explicit id and no names yet; add them with
    /// [`long`](Arg::long) and [`short`](Arg::short). Use this for options
    /// that only have a short name: `Arg::with_id("v").short('v')`.
    pub fn with_id(id: impl Into<String>) -> Arg<bool> {
        Arg::flag(id.into())
    }

    /// A positional argument parsed with [`FromStr`]. `name` is both the id
    /// and the placeholder shown in help: `Arg::positional::<PathBuf>("FILE")`.
    pub fn positional<U>(name: impl Into<String>) -> Arg<Option<U>>
    where
        U: FromStr + Any + Clone + Send + Sync,
        U::Err: Display,
    {
        Arg::positional_with(name, U::from_str)
    }

    /// A positional argument parsed with `f`.
    pub fn positional_with<U, F, E>(name: impl Into<String>, f: F) -> Arg<Option<U>>
    where
        U: Any + Clone + Send + Sync,
        F: Fn(&str) -> Result<U, E> + Send + Sync + 'static,
        E: Display,
    {
        let name = name.into();
        let mut a = Arg::with_id(name.clone()).value_with(f);
        a.def.positional = true;
        if let Some(v) = a.def.value.as_mut() {
            v.name = name;
        }
        a
    }

    /// A positional argument parsed from the raw [`OsStr`] with `f`.
    pub fn positional_os_with<U, F, E>(name: impl Into<String>, f: F) -> Arg<Option<U>>
    where
        U: Any + Clone + Send + Sync,
        F: Fn(&OsStr) -> Result<U, E> + Send + Sync + 'static,
        E: Display,
    {
        let name = name.into();
        let mut a = Arg::with_id(name.clone()).value_os_with(f);
        a.def.positional = true;
        if let Some(v) = a.def.value.as_mut() {
            v.name = name;
        }
        a
    }

    /// A positional argument taking the raw [`OsString`].
    pub fn positional_os(name: impl Into<String>) -> Arg<Option<OsString>> {
        Arg::positional_os_with(name, |s: &OsStr| {
            Ok::<_, std::convert::Infallible>(s.to_os_string())
        })
    }

    /// Turn the flag into a counter: `-vvv` gives `3`.
    pub fn count(mut self) -> Arg<usize> {
        self.def.count = true;
        self.retype(ex_count)
    }

    /// Make the argument take a value parsed with [`FromStr`].
    pub fn value<U>(self) -> Arg<Option<U>>
    where
        U: FromStr + Any + Clone + Send + Sync,
        U::Err: Display,
    {
        self.value_with(U::from_str)
    }

    /// Make the argument take a value parsed with `f`.
    pub fn value_with<U, F, E>(self, f: F) -> Arg<Option<U>>
    where
        U: Any + Clone + Send + Sync,
        F: Fn(&str) -> Result<U, E> + Send + Sync + 'static,
        E: Display,
    {
        self.value_parser(str_parser(f))
    }

    /// Make the argument take a raw [`OsString`] value (never fails on
    /// non-Unicode input).
    pub fn value_os(self) -> Arg<Option<OsString>> {
        self.value_os_with(|s: &OsStr| Ok::<_, std::convert::Infallible>(s.to_os_string()))
    }

    /// Make the argument take a value parsed from the raw [`OsStr`] with `f`.
    pub fn value_os_with<U, F, E>(self, f: F) -> Arg<Option<U>>
    where
        U: Any + Clone + Send + Sync,
        F: Fn(&OsStr) -> Result<U, E> + Send + Sync + 'static,
        E: Display,
    {
        self.value_parser(os_parser(f))
    }

    fn value_parser<U: Any + Clone + Send + Sync>(mut self, parser: ValueParser) -> Arg<Option<U>> {
        self.def.value = Some(ValueDef {
            name: value_name_from_id(&self.def.id),
            parser,
            default: None,
            default_missing: None,
            possible: Vec::new(),
            #[cfg(feature = "env")]
            env: None,
            completer: None,
        });
        self.retype(ex_option::<U>)
    }
}

impl<U: Any + Clone + Send + Sync> Arg<Option<U>> {
    /// The argument must be present; [`Matches::get`](crate::Matches::get)
    /// then returns `U` directly.
    pub fn required(mut self) -> Arg<U> {
        self.def.required = true;
        self.retype(ex_required::<U>)
    }

    /// Use `v` when the argument is absent. `v` is shown in help through
    /// its [`Display`] impl.
    pub fn default(self, v: U) -> Arg<U>
    where
        U: Display,
    {
        let text = v.to_string();
        self.default_with(v, text)
    }

    /// Use `v` when the argument is absent, showing `text` in help.
    pub fn default_with(mut self, v: U, text: impl Into<String>) -> Arg<U> {
        if let Some(vd) = &mut self.def.value {
            vd.default = Some((maker(v), text.into()));
        }
        self.retype(ex_required::<U>)
    }

    /// Make the value optional: `--color` alone yields `v`, `--color=never`
    /// yields `never`. A value can only be attached (`--color=x`, `-cx`),
    /// never given as the next argument, so `--color auto` treats `auto` as
    /// a positional argument.
    ///
    /// Call this before [`required`](Arg::required), [`default`](Arg::default)
    /// or [`many`](Arg::many).
    pub fn default_missing(mut self, v: U) -> Arg<Option<U>>
    where
        U: Display,
    {
        let text = v.to_string();
        if let Some(vd) = &mut self.def.value {
            vd.default_missing = Some((maker(v), text));
        }
        self
    }

    /// Like [`default_missing`](Arg::default_missing), showing `text` in help.
    pub fn default_missing_with(mut self, v: U, text: impl Into<String>) -> Arg<Option<U>> {
        if let Some(vd) = &mut self.def.value {
            vd.default_missing = Some((maker(v), text.into()));
        }
        self
    }

    /// Restrict the accepted values. Checked before parsing, and listed in
    /// help.
    pub fn possible<I, S>(mut self, values: I) -> Arg<Option<U>>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        if let Some(vd) = &mut self.def.value {
            vd.possible = values.into_iter().map(Into::into).collect();
        }
        self
    }

    /// Provide shell completion candidates for the value at runtime. `f`
    /// receives the text typed so far and returns the candidates (it may
    /// ignore the prefix; shells filter). Used by `hasami-complete`.
    pub fn complete_with<F>(mut self, f: F) -> Arg<Option<U>>
    where
        F: Fn(&str) -> Vec<String> + Send + Sync + 'static,
    {
        if let Some(vd) = &mut self.def.value {
            vd.completer = Some(Arc::new(f));
        }
        self
    }

    /// Fall back to the environment variable `var` when the argument is
    /// absent from the command line.
    #[cfg(feature = "env")]
    pub fn env(mut self, var: impl Into<String>) -> Arg<Option<U>> {
        if let Some(vd) = &mut self.def.value {
            vd.env = Some(var.into());
        }
        self
    }

    /// Allow the argument to repeat (`-f a -f b`), collecting every value.
    /// For a positional argument this collects all remaining positionals.
    pub fn many(mut self) -> Arg<Vec<U>> {
        self.def.many = true;
        self.retype(ex_vec::<U>)
    }
}

impl<U: Any + Clone + Send + Sync> Arg<Vec<U>> {
    /// A repeated argument that must appear at least once.
    pub fn required(mut self) -> Arg<Vec<U>> {
        self.def.required = true;
        self
    }
}
