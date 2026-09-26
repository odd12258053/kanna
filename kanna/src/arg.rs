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

/// The broad category of a value's Rust type, recorded when the argument
/// is defined so that generators (schemas, tool definitions) can pick a
/// JSON type without knowing the type itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValueType {
    /// Any built-in integer type.
    Integer,
    /// `f32` or `f64`.
    Float,
    /// `bool`.
    Boolean,
    /// `String` or `OsString`.
    String,
    /// `PathBuf`.
    Path,
    /// Anything else (a user type with `FromStr`).
    Other,
}

fn value_type_of<U: Any>() -> ValueType {
    use std::any::TypeId;
    let id = TypeId::of::<U>();
    let ints = [
        TypeId::of::<i8>(),
        TypeId::of::<i16>(),
        TypeId::of::<i32>(),
        TypeId::of::<i64>(),
        TypeId::of::<i128>(),
        TypeId::of::<isize>(),
        TypeId::of::<u8>(),
        TypeId::of::<u16>(),
        TypeId::of::<u32>(),
        TypeId::of::<u64>(),
        TypeId::of::<u128>(),
        TypeId::of::<usize>(),
    ];
    if ints.contains(&id) {
        ValueType::Integer
    } else if id == TypeId::of::<f32>() || id == TypeId::of::<f64>() {
        ValueType::Float
    } else if id == TypeId::of::<bool>() {
        ValueType::Boolean
    } else if id == TypeId::of::<String>() || id == TypeId::of::<OsString>() {
        ValueType::String
    } else if id == TypeId::of::<std::path::PathBuf>() {
        ValueType::Path
    } else {
        ValueType::Other
    }
}

/// How a value-taking argument behaves.
#[derive(Clone)]
pub(crate) struct ValueDef {
    pub(crate) name: String,
    pub(crate) kind: ValueType,
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
    /// Split each occurrence on this character (`-f a,b,c`).
    pub(crate) delimiter: Option<char>,
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
    /// Long names that are accepted and listed in help.
    pub(crate) visible_aliases: Vec<String>,
    /// Repeating a single-value argument replaces the value instead of
    /// being an error.
    pub(crate) last_wins: bool,
    /// Each occurrence takes every following non-option argument.
    pub(crate) greedy: bool,
    /// Once this positional starts, everything left is a value for it.
    pub(crate) trailing: bool,
    /// Help shown by `--help`; `help` is used by `-h`.
    pub(crate) long_help: Option<String>,
    /// Section title in help instead of `Options`/`Arguments`.
    pub(crate) heading: Option<String>,
    /// Constraints involving other arguments.
    pub(crate) relations: Vec<Relation>,
}

/// A constraint between one argument and another, declared on the
/// argument with [`Arg::requires`], [`Arg::conflicts_with`],
/// [`Arg::required_unless`], [`Arg::required_if_eq`] or
/// [`Arg::requires_if`]. Ids name the other argument.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Relation {
    /// The other argument must be present when this one is.
    Requires(String),
    /// The other argument cannot be present when this one is.
    ConflictsWith(String),
    /// This argument is required unless the other is present.
    RequiredUnless(String),
    /// This argument is required when the other has the given raw value.
    RequiredIfEq(String, String),
    /// When this argument has the given raw value, the other is required.
    RequiresIf(String, String),
}

impl Relation {
    /// The id of the other argument.
    pub fn other(&self) -> &str {
        match self {
            Relation::Requires(id)
            | Relation::ConflictsWith(id)
            | Relation::RequiredUnless(id)
            | Relation::RequiredIfEq(id, _)
            | Relation::RequiresIf(_, id) => id,
        }
    }
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
            visible_aliases: Vec::new(),
            last_wins: false,
            greedy: false,
            trailing: false,
            long_help: None,
            heading: None,
            relations: Vec::new(),
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
    /// Additional long names that are not shown in help.
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }
    /// Additional long names that are shown in help.
    pub fn visible_aliases(&self) -> &[String] {
        &self.visible_aliases
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
    /// May a single-value argument be repeated, the last value winning?
    pub fn is_last_wins(&self) -> bool {
        self.last_wins
    }
    /// Does each occurrence take every following non-option argument?
    pub fn is_greedy(&self) -> bool {
        self.greedy
    }
    /// Does this positional swallow everything after its first value?
    pub fn is_trailing(&self) -> bool {
        self.trailing
    }
    /// The longer help text shown by `--help`, if one was set.
    pub fn long_help(&self) -> Option<&str> {
        self.long_help.as_deref()
    }
    /// The custom help section this argument is listed under.
    pub fn help_heading(&self) -> Option<&str> {
        self.heading.as_deref()
    }
    /// The category of the value's Rust type, for generators.
    pub fn value_type(&self) -> Option<ValueType> {
        self.value.as_ref().map(|v| v.kind)
    }
    /// The character each value is split on, if any.
    pub fn delimiter(&self) -> Option<char> {
        self.value.as_ref().and_then(|v| v.delimiter)
    }
    /// The constraints declared on this argument towards other arguments.
    pub fn relations(&self) -> &[Relation] {
        &self.relations
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
        out.extend(self.visible_aliases.iter().map(|l| format!("--{l}")));
        out
    }

    pub(crate) fn matches_long(&self, name: &str) -> bool {
        self.long.as_deref() == Some(name)
            || self.aliases.iter().any(|a| a == name)
            || self.visible_aliases.iter().any(|a| a == name)
    }

    /// Every long name, canonical first.
    pub(crate) fn long_names(&self) -> impl Iterator<Item = &str> {
        self.long
            .iter()
            .chain(&self.aliases)
            .chain(&self.visible_aliases)
            .map(String::as_str)
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
/// use kanna::{Arg, Command};
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

    /// Add an alternative long name that is listed in help as
    /// `[aliases: name]`.
    pub fn visible_alias(mut self, name: impl Into<String>) -> Self {
        self.def.visible_aliases.push(name.into());
        self
    }

    /// A longer help text shown by `--help`; `-h` keeps showing
    /// [`help`](Arg::help).
    pub fn long_help(mut self, text: impl Into<String>) -> Self {
        self.def.long_help = Some(text.into());
        self
    }

    /// List the argument under its own section title in help instead of
    /// `Options` or `Arguments`. Arguments sharing a title are grouped.
    pub fn help_heading(mut self, title: impl Into<String>) -> Self {
        self.def.heading = Some(title.into());
        self
    }

    /// Allow the argument to be given more than once, the last value
    /// winning. By default a single-value argument or a flag given twice
    /// is an error (see [`Command::args_override_self`](crate::Command::args_override_self)).
    pub fn last_wins(mut self) -> Self {
        self.def.last_wins = true;
        self
    }

    /// When this argument is present, `other` must be too.
    pub fn requires<U>(mut self, other: &Arg<U>) -> Self {
        self.def
            .relations
            .push(Relation::Requires(other.id().to_owned()));
        self
    }

    /// This argument and `other` cannot be given together.
    pub fn conflicts_with<U>(mut self, other: &Arg<U>) -> Self {
        self.def
            .relations
            .push(Relation::ConflictsWith(other.id().to_owned()));
        self
    }

    /// This argument is required unless `other` is present. Several calls
    /// accumulate: any one of the named arguments lifts the requirement.
    pub fn required_unless<U>(mut self, other: &Arg<U>) -> Self {
        self.def
            .relations
            .push(Relation::RequiredUnless(other.id().to_owned()));
        self
    }

    /// This argument is required when `other` was given the raw value
    /// `value` on the command line.
    pub fn required_if_eq<U>(mut self, other: &Arg<U>, value: impl Into<String>) -> Self {
        self.def
            .relations
            .push(Relation::RequiredIfEq(other.id().to_owned(), value.into()));
        self
    }

    /// When this argument was given the raw value `value`, `other` must be
    /// present.
    pub fn requires_if<U>(mut self, value: impl Into<String>, other: &Arg<U>) -> Self {
        self.def
            .relations
            .push(Relation::RequiresIf(value.into(), other.id().to_owned()));
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
            kind: value_type_of::<U>(),
            parser,
            default: None,
            default_missing: None,
            possible: Vec::new(),
            #[cfg(feature = "env")]
            env: None,
            completer: None,
            delimiter: None,
        });
        self.retype(ex_option::<U>)
    }

    /// Make the argument take one of the values of a [`ValueEnum`]; the
    /// accepted names are listed in help and checked before parsing, and
    /// both come from the same definition, so they cannot drift apart.
    pub fn value_enum<U: ValueEnum>(self) -> Arg<Option<U>> {
        self.value_with(|s| U::from_name(s).ok_or("unknown value"))
            .possible(U::names())
    }

    /// A positional argument taking one of the values of a [`ValueEnum`].
    pub fn positional_enum<U: ValueEnum>(name: impl Into<String>) -> Arg<Option<U>> {
        Arg::positional_with(name, |s| U::from_name(s).ok_or("unknown value")).possible(U::names())
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
    /// ignore the prefix; shells filter). Used by `kanna-complete`.
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

    /// Let each occurrence take every following argument that does not
    /// look like an option: `--exec cmd arg arg`. An attached value
    /// (`--exec=cmd`) limits that occurrence to one value.
    pub fn greedy(mut self) -> Arg<Vec<U>> {
        self.def.greedy = true;
        self
    }

    /// Split every value on `c`: `-f a,b,c` yields three values. Splitting
    /// happens before parsing, so each piece must be valid on its own.
    pub fn delimiter(mut self, c: char) -> Arg<Vec<U>> {
        if let Some(v) = &mut self.def.value {
            v.delimiter = Some(c);
        }
        self
    }

    /// For a repeated positional: once its first value is seen, every
    /// remaining argument is a value for it, options included
    /// (`run prog -x --y`). Equivalent to an implicit `--`.
    pub fn trailing(mut self) -> Arg<Vec<U>> {
        self.def.trailing = true;
        self
    }
}

/// A fixed set of named values, for options that accept one of a list.
///
/// Implement it with [`value_enum!`](crate::value_enum) (no proc-macro) or
/// `#[derive(ValueEnum)]` (feature `derive`), then use
/// [`Arg::value_enum`] or the `value_enum` field setting. The names shown
/// in help and the names accepted come from the same list.
pub trait ValueEnum: Clone + Send + Sync + 'static {
    /// Every value, in the order shown in help.
    const VALUES: &'static [Self];

    /// The name this value is written as on the command line.
    fn name(&self) -> &'static str;

    /// The value spelled `name`, if any.
    fn from_name(name: &str) -> Option<Self> {
        Self::VALUES.iter().find(|v| v.name() == name).cloned()
    }

    /// All names, in order.
    fn names() -> Vec<String> {
        Self::VALUES.iter().map(|v| v.name().to_owned()).collect()
    }
}

/// Define an enum that implements [`ValueEnum`], `FromStr` and `Display`
/// from one list of `Variant = "name"` pairs.
///
/// ```
/// kanna::value_enum! {
///     /// How much to say
///     #[derive(Debug, Clone, Copy, PartialEq, Eq)]
///     pub enum Level { Quiet = "quiet", Normal = "normal", Loud = "loud" }
/// }
///
/// use kanna::{Arg, Command, ValueEnum};
/// let level = Arg::new("level").value_enum::<Level>().default(Level::Normal);
/// let cmd = Command::new("x").arg(&level);
/// assert_eq!(cmd.try_parse_args(["--level", "loud"]).unwrap().get(&level), Level::Loud);
/// assert!(cmd.try_parse_args(["--level", "shout"]).is_err());
/// assert_eq!(Level::names(), ["quiet", "normal", "loud"]);
/// ```
///
/// The enum must derive or implement `Clone`.
#[macro_export]
macro_rules! value_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $Name:ident {
            $( $(#[$vmeta:meta])* $Variant:ident = $name:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $Name {
            $( $(#[$vmeta])* $Variant ),+
        }

        impl $crate::ValueEnum for $Name {
            const VALUES: &'static [$Name] = &[ $( $Name::$Variant ),+ ];

            fn name(&self) -> &'static str {
                match self {
                    $( $Name::$Variant => $name ),+
                }
            }
        }

        impl ::std::str::FromStr for $Name {
            type Err = String;

            fn from_str(s: &str) -> Result<$Name, String> {
                <$Name as $crate::ValueEnum>::from_name(s)
                    .ok_or_else(|| $crate::__macro::unknown_enum_value(s, &<$Name as $crate::ValueEnum>::names()))
            }
        }

        impl ::std::fmt::Display for $Name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(<$Name as $crate::ValueEnum>::name(self))
            }
        }
    };
}
