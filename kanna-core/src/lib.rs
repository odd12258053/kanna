//! # kanna-core
//!
//! A dependency-free, panic-free, non-Unicode-safe command line **lexer** that
//! follows GNU/POSIX conventions. It is the foundation every other `kanna`
//! layer is built on, and it is usable on its own as a lightweight imperative
//! parser in the spirit of `lexopt`.
//!
//! The crate is deliberately tiny: one source file, no `unsafe`, no
//! dependencies, no proc-macros. It can be audited or vendored in an afternoon.
//!
//! ## What it accepts
//!
//! | Input                | Yields                                            |
//! |----------------------|---------------------------------------------------|
//! | `--opt`              | [`Long("opt")`](Arg::Long)                        |
//! | `--opt=val`          | `Long("opt")` with attached value `val`           |
//! | `--opt=`             | `Long("opt")` with attached empty value           |
//! | `--opt val`          | `Long("opt")`, then [`Parser::value`] takes `val` |
//! | `-o`                 | [`Short('o')`](Arg::Short)                       |
//! | `-oval`, `-o=val`    | `Short('o')` with attached value `val`            |
//! | `-abc`               | `Short('a')`, `Short('b')`, `Short('c')`          |
//! | `-`                  | [`Value("-")`](Arg::Value)                        |
//! | `--`                 | Nothing; everything after it is a `Value`         |
//! | anything else        | `Value(...)`                                      |
//!
//! Values are [`OsString`]s, so arguments that are not valid Unicode are
//! passed through untouched. Converting to `String` or parsing into a typed
//! value is explicit and fallible through [`ValueExt`].
//!
//! ## Example
//!
//! ```no_run
//! use kanna_core::prelude::*;
//!
//! fn main() -> Result<(), kanna_core::Error> {
//!     let mut number = 1u32;
//!     let mut shout = false;
//!     let mut thing: Option<String> = None;
//!
//!     let mut parser = Parser::from_env();
//!     while let Some(arg) = parser.next()? {
//!         match arg {
//!             Short('n') | Long("number") => number = parser.value()?.parse()?,
//!             Long("shout") => shout = true,
//!             Value(v) => thing = Some(v.string()?),
//!             _ => return Err(arg.unexpected()),
//!         }
//!     }
//!
//!     let thing = thing.ok_or("missing argument THING")?;
//!     let mut message = format!("Hello {thing}");
//!     if shout {
//!         message = message.to_uppercase();
//!     }
//!     for _ in 0..number {
//!         println!("{message}");
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Error recovery
//!
//! Every error returned by [`Parser::next`] leaves the parser in a consistent
//! state, so a caller may keep calling `next()` to collect several problems
//! before giving up.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::error::Error as StdError;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::str::FromStr;

/// Convenience re-exports: `use kanna_core::prelude::*;` brings in the
/// parser, the argument variants and the value extension trait.
pub mod prelude {
    pub use super::Arg::*;
    pub use super::{Arg, Parser, ValueExt};
}

/// One lexed command line argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg<'a> {
    /// A short option such as `-n`. The cluster `-abc` yields three of these.
    ///
    /// A byte that is not valid Unicode inside a short cluster is reported as
    /// `U+FFFD REPLACEMENT CHARACTER`; the raw bytes are still available as an
    /// attached value through [`Parser::value`].
    Short(char),
    /// A long option such as `--number`. The name never includes the leading
    /// `--` nor an attached `=value`.
    ///
    /// The name borrows from the [`Parser`], so it must be matched before the
    /// parser is used again. Pattern matching (`Long("number") => ...`) does
    /// exactly that.
    Long(&'a str),
    /// A positional argument: anything that does not start with `-`, the
    /// lone `-`, or any argument after `--`.
    Value(OsString),
}

impl Arg<'_> {
    /// Convert an argument the caller did not expect into an [`Error`].
    ///
    /// Options become [`Error::UnexpectedOption`], values become
    /// [`Error::UnexpectedArgument`].
    pub fn unexpected(self) -> Error {
        match self {
            Arg::Short(c) => Error::UnexpectedOption(format!("-{c}")),
            Arg::Long(name) => Error::UnexpectedOption(format!("--{name}")),
            Arg::Value(v) => Error::UnexpectedArgument(v),
        }
    }
}

/// Errors produced by the lexer and by [`ValueExt`].
///
/// The enum is `#[non_exhaustive]`; match with a wildcard arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An option that requires a value was given none.
    MissingValue {
        /// The option (`-n` or `--number`) that was missing its value, when known.
        option: Option<String>,
    },
    /// An option the caller did not recognise (rendered as `-n` or `--name`).
    UnexpectedOption(String),
    /// A positional argument the caller did not recognise.
    UnexpectedArgument(OsString),
    /// An option was given an attached value (`--flag=x`) but the caller never
    /// asked for one.
    UnexpectedValue {
        /// The option that received the value.
        option: String,
        /// The value that was attached.
        value: OsString,
    },
    /// A value was not valid Unicode where Unicode was required.
    NonUnicodeValue(OsString),
    /// A value could not be parsed into the requested type.
    ParsingFailed {
        /// The value that failed to parse.
        value: String,
        /// The error reported by the type's parser.
        error: Box<dyn StdError + Send + Sync + 'static>,
    },
    /// An arbitrary error, typically produced by application code.
    Custom(Box<dyn StdError + Send + Sync + 'static>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingValue { option: Some(opt) } => {
                write!(f, "missing argument for option '{opt}'")
            }
            Error::MissingValue { option: None } => write!(f, "missing argument"),
            Error::UnexpectedOption(opt) => write!(f, "invalid option '{opt}'"),
            Error::UnexpectedArgument(v) => write!(f, "unexpected argument {v:?}"),
            Error::UnexpectedValue { option, value } => {
                write!(f, "unexpected argument for option '{option}': {value:?}")
            }
            Error::NonUnicodeValue(v) => write!(f, "argument is invalid unicode: {v:?}"),
            Error::ParsingFailed { value, error } => {
                write!(f, "cannot parse argument {value:?}: {error}")
            }
            Error::Custom(e) => write!(f, "{e}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::ParsingFailed { error, .. } | Error::Custom(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

impl From<Box<dyn StdError + Send + Sync + 'static>> for Error {
    fn from(e: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        Error::Custom(e)
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::Custom(msg.into())
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::Custom(msg.into())
    }
}

/// Lexer state between two calls to [`Parser::next`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Nothing pending: the next raw argument starts a new token.
    Ready,
    /// Inside a short option cluster. The `usize` is the byte offset into the
    /// current argument of the next unread character.
    Shorts(usize),
    /// A `--long=value` was seen and `value` (starting at the byte offset) has
    /// not been consumed yet.
    Attached(usize),
    /// `--` was seen: every remaining argument is a positional value.
    Positionals,
}

/// Which option was returned most recently, for error messages.
#[derive(Debug, Clone, Copy)]
enum Last {
    None,
    Short(char),
    /// The name lives in `Parser::long`.
    Long,
}

/// The command line lexer.
///
/// Create one with [`Parser::from_env`], [`Parser::from_iter`] or
/// [`Parser::from_args`], then drive it with [`Parser::next`] and the value
/// accessors ([`value`](Parser::value), [`optional_value`](Parser::optional_value),
/// [`values`](Parser::values), [`raw_args`](Parser::raw_args)).
#[derive(Debug)]
pub struct Parser {
    args: std::vec::IntoIter<OsString>,
    bin_name: Option<OsString>,
    state: State,
    /// The argument currently being lexed (meaningful in `Shorts`/`Attached`).
    cur: OsString,
    /// Storage for the last long option name so that `Arg::Long` can borrow it.
    long: String,
    last: Last,
}

impl Parser {
    /// Create a parser over the process arguments. The first argument is
    /// treated as the binary name (see [`bin_name`](Parser::bin_name)).
    pub fn from_env() -> Parser {
        Parser::from_iter(std::env::args_os())
    }

    /// Create a parser from an iterator whose **first item is the binary
    /// name**, exactly like `std::env::args_os()`.
    #[allow(clippy::should_implement_trait)] // deliberately not `FromIterator`: it is fallible-free but semantically distinct
    pub fn from_iter<I>(args: I) -> Parser
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        let mut args = args.into_iter();
        let bin_name = args.next().map(Into::into);
        let mut parser = Parser::from_args(args);
        parser.bin_name = bin_name;
        parser
    }

    /// Create a parser from an iterator of arguments **without** a binary name.
    pub fn from_args<I>(args: I) -> Parser
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
        Parser {
            args: args.into_iter(),
            bin_name: None,
            state: State::Ready,
            cur: OsString::new(),
            long: String::new(),
            last: Last::None,
        }
    }

    /// The binary name, if one was supplied and it is valid Unicode.
    pub fn bin_name(&self) -> Option<&str> {
        self.bin_name.as_deref().and_then(OsStr::to_str)
    }

    /// The binary name, if one was supplied, as an [`OsStr`].
    pub fn bin_name_os(&self) -> Option<&OsStr> {
        self.bin_name.as_deref()
    }

    /// Lex the next argument.
    ///
    /// Returns `Ok(None)` once all arguments are consumed. Calling it again
    /// afterwards keeps returning `Ok(None)`.
    ///
    /// # Errors
    ///
    /// * [`Error::UnexpectedValue`] if the previous option carried a
    ///   `=value` that was never consumed with [`value`](Parser::value) or
    ///   [`optional_value`](Parser::optional_value).
    /// * [`Error::UnexpectedOption`] if a long option name is not valid
    ///   Unicode.
    ///
    /// After an error the parser is in a consistent state and may be used
    /// again.
    ///
    /// This is not an [`Iterator`] because the returned [`Arg`] borrows the
    /// parser and because the natural loop is `while let Some(arg) = p.next()?`.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<Arg<'_>>, Error> {
        loop {
            match self.state {
                State::Attached(pos) => {
                    self.state = State::Ready;
                    let value = os_suffix(&self.cur, pos)?;
                    let option = self.last_option().unwrap_or_default();
                    return Err(Error::UnexpectedValue { option, value });
                }
                State::Shorts(pos) => {
                    let bytes = self.cur.as_encoded_bytes();
                    match bytes.get(pos..) {
                        Some(rest) if !rest.is_empty() => {
                            let (ch, len) = decode_char(rest);
                            self.state = State::Shorts(pos + len);
                            self.last = Last::Short(ch);
                            return Ok(Some(Arg::Short(ch)));
                        }
                        _ => self.state = State::Ready,
                    }
                }
                State::Ready | State::Positionals => {}
            }

            let Some(arg) = self.args.next() else {
                return Ok(None);
            };
            if self.state == State::Positionals {
                return Ok(Some(Arg::Value(arg)));
            }

            let bytes = arg.as_encoded_bytes();
            if bytes == b"--" {
                self.state = State::Positionals;
                continue;
            }
            if let Some(rest) = bytes.strip_prefix(b"--") {
                // `--name` or `--name=value`. The name must be valid Unicode;
                // splitting at the ASCII `=` can never cut a multi-byte
                // sequence, so `from_utf8` on the slice is sound on every
                // platform.
                let name_len = rest.iter().position(|&b| b == b'=').unwrap_or(rest.len());
                match std::str::from_utf8(&rest[..name_len]) {
                    Ok(name) => {
                        self.long.clear();
                        self.long.push_str(name);
                    }
                    Err(_) => {
                        let shown = String::from_utf8_lossy(&bytes[..2 + name_len]).into_owned();
                        return Err(Error::UnexpectedOption(shown));
                    }
                }
                self.state = if name_len < rest.len() {
                    State::Attached(2 + name_len + 1)
                } else {
                    State::Ready
                };
                self.last = Last::Long;
                self.cur = arg;
                return Ok(Some(Arg::Long(&self.long)));
            }
            if bytes.len() > 1 && bytes[0] == b'-' {
                self.state = State::Shorts(1);
                self.cur = arg;
                continue;
            }
            return Ok(Some(Arg::Value(arg)));
        }
    }

    /// Take the value of the option that was just returned.
    ///
    /// The value is the attached text if there is any (`--opt=val`, `-oval`,
    /// `-o=val`), otherwise the next raw argument, whatever it looks like:
    /// `--opt --weird` gives the value `--weird`, matching GNU `getopt`.
    ///
    /// # Errors
    ///
    /// [`Error::MissingValue`] if there is neither an attached value nor a
    /// following argument.
    pub fn value(&mut self) -> Result<OsString, Error> {
        if let Some(v) = self.optional_value()? {
            return Ok(v);
        }
        match self.args.next() {
            Some(v) => Ok(v),
            None => Err(Error::MissingValue {
                option: self.last_option(),
            }),
        }
    }

    /// Take the value of the option that was just returned, but only if it is
    /// attached (`--color=always`, `-cnever`, `-c=auto`).
    ///
    /// This implements options with an optional argument such as
    /// `--color[=WHEN]`. `--opt=` yields `Some("")`.
    ///
    /// # Errors
    ///
    /// [`Error::NonUnicodeValue`] only on platforms that are neither Unix nor
    /// Windows, when the attached value is not valid Unicode.
    pub fn optional_value(&mut self) -> Result<Option<OsString>, Error> {
        match self.state {
            State::Attached(pos) => {
                self.state = State::Ready;
                os_suffix(&self.cur, pos).map(Some)
            }
            State::Shorts(pos) => {
                self.state = State::Ready;
                let bytes = self.cur.as_encoded_bytes();
                match bytes.get(pos) {
                    None => Ok(None),
                    Some(b'=') => os_suffix(&self.cur, pos + 1).map(Some),
                    Some(_) => os_suffix(&self.cur, pos).map(Some),
                }
            }
            State::Ready | State::Positionals => Ok(None),
        }
    }

    /// Gather several values for the option that was just returned, e.g.
    /// `--exec echo hello world`.
    ///
    /// The iterator yields the attached value if any, then every following
    /// raw argument that does not look like an option (does not start with
    /// `-`, or is exactly `-`). It stops before the next option, before `--`,
    /// or at the end of the arguments.
    ///
    /// A value attached with `=` limits the iterator to that single value:
    /// `--opt=a b` yields only `a`, while `--opt a b` and `-oa b` yield `a`
    /// and `b`.
    ///
    /// # Errors
    ///
    /// [`Error::MissingValue`] if not even one value is available. The
    /// iterator itself never fails.
    pub fn values(&mut self) -> Result<ValuesIter<'_>, Error> {
        let has_attached = self.has_pending_value();
        if has_attached || self.next_is_value() {
            Ok(ValuesIter {
                parser: self,
                done: false,
            })
        } else {
            Err(Error::MissingValue {
                option: self.last_option(),
            })
        }
    }

    /// Take over the remaining raw arguments without lexing them.
    ///
    /// Useful for `cmd -- args...` style pass-through, or for handing the
    /// tail of the command line to another parser. Arguments consumed through
    /// the returned iterator are gone; anything left when it is dropped will
    /// be lexed normally by the next call to [`next`](Parser::next).
    ///
    /// # Errors
    ///
    /// [`Error::UnexpectedValue`] if the previous option carried an attached
    /// value (`--opt=x`, `-ox`) that was not consumed: it is ambiguous whether
    /// that text belongs to the option or to the raw tail.
    pub fn raw_args(&mut self) -> Result<RawArgs<'_>, Error> {
        if let Some(value) = self.optional_value()? {
            let option = self.last_option().unwrap_or_default();
            return Err(Error::UnexpectedValue { option, value });
        }
        Ok(RawArgs {
            args: &mut self.args,
        })
    }

    /// Like [`raw_args`](Parser::raw_args), but returns `None` instead of an
    /// error when an attached value is pending, leaving it in place.
    pub fn try_raw_args(&mut self) -> Option<RawArgs<'_>> {
        if self.has_pending_value() {
            return None;
        }
        self.state = match self.state {
            State::Positionals => State::Positionals,
            _ => State::Ready,
        };
        Some(RawArgs {
            args: &mut self.args,
        })
    }

    /// The option most recently returned by [`next`](Parser::next), rendered
    /// as `-n` or `--number`. `None` if no option was returned yet.
    fn last_option(&self) -> Option<String> {
        match self.last {
            Last::None => None,
            Last::Short(c) => Some(format!("-{c}")),
            Last::Long => Some(format!("--{}", self.long)),
        }
    }

    /// Is there attached text (`=value` or the tail of a short cluster) that
    /// has not been consumed?
    fn has_pending_value(&self) -> bool {
        match self.state {
            State::Attached(_) => true,
            State::Shorts(pos) => pos < self.cur.as_encoded_bytes().len(),
            State::Ready | State::Positionals => false,
        }
    }

    /// Does the next raw argument look like a value rather than an option?
    fn next_is_value(&self) -> bool {
        match self.args.as_slice().first() {
            Some(arg) => looks_like_value(arg),
            None => false,
        }
    }
}

/// `true` for arguments that [`Parser::values`] accepts: anything not starting
/// with `-`, plus the lone `-`.
fn looks_like_value(arg: &OsStr) -> bool {
    let bytes = arg.as_encoded_bytes();
    bytes.first() != Some(&b'-') || bytes.len() == 1
}

/// Iterator returned by [`Parser::values`].
#[derive(Debug)]
pub struct ValuesIter<'a> {
    parser: &'a mut Parser,
    done: bool,
}

impl Iterator for ValuesIter<'_> {
    type Item = OsString;

    fn next(&mut self) -> Option<OsString> {
        if self.done {
            return None;
        }
        let p = &mut *self.parser;
        match p.state {
            State::Attached(pos) => {
                // `--opt=value`: a single value only.
                p.state = State::Ready;
                self.done = true;
                return os_suffix(&p.cur, pos).ok();
            }
            State::Shorts(pos) => {
                p.state = State::Ready;
                let bytes = p.cur.as_encoded_bytes();
                match bytes.get(pos) {
                    None => {}
                    Some(b'=') => {
                        // `-o=value`: a single value only.
                        self.done = true;
                        return os_suffix(&p.cur, pos + 1).ok();
                    }
                    Some(_) => {
                        // `-ovalue`: first value, more may follow.
                        return os_suffix(&p.cur, pos).ok();
                    }
                }
            }
            State::Ready | State::Positionals => {}
        }
        if p.next_is_value() {
            p.args.next()
        } else {
            self.done = true;
            None
        }
    }
}

/// Iterator returned by [`Parser::raw_args`] and [`Parser::try_raw_args`].
///
/// Besides being an iterator it offers [`peek`](RawArgs::peek),
/// [`next_if`](RawArgs::next_if) and [`as_slice`](RawArgs::as_slice) so callers
/// can stop consuming at a boundary of their choosing.
#[derive(Debug)]
pub struct RawArgs<'a> {
    args: &'a mut std::vec::IntoIter<OsString>,
}

impl RawArgs<'_> {
    /// Look at the next raw argument without consuming it.
    pub fn peek(&self) -> Option<&OsStr> {
        self.args.as_slice().first().map(OsString::as_os_str)
    }

    /// Consume the next raw argument only if `pred` accepts it.
    pub fn next_if(&mut self, pred: impl FnOnce(&OsStr) -> bool) -> Option<OsString> {
        if pred(self.peek()?) {
            self.args.next()
        } else {
            None
        }
    }

    /// All remaining raw arguments.
    pub fn as_slice(&self) -> &[OsString] {
        self.args.as_slice()
    }
}

impl Iterator for RawArgs<'_> {
    type Item = OsString;

    fn next(&mut self) -> Option<OsString> {
        self.args.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.args.size_hint()
    }
}

impl ExactSizeIterator for RawArgs<'_> {}

/// Decode one character from the start of `bytes` (which must be non-empty).
///
/// Returns the character and the number of bytes it occupied. Ill-formed
/// input yields `U+FFFD` and advances past the maximal ill-formed unit: a
/// WTF-8 encoded surrogate (as produced by Windows for unpaired UTF-16
/// surrogates) counts as one three-byte unit so that offsets always stay on
/// code point boundaries.
fn decode_char(bytes: &[u8]) -> (char, usize) {
    match std::str::from_utf8(bytes) {
        Ok(s) => first_char(s),
        Err(e) if e.valid_up_to() > 0 => {
            // The prefix is valid; `from_utf8` on it cannot fail.
            std::str::from_utf8(&bytes[..e.valid_up_to()]).map_or(('\u{FFFD}', 1), first_char)
        }
        Err(e) => {
            let len = if is_wtf8_surrogate(bytes) {
                3
            } else {
                e.error_len().unwrap_or(bytes.len()).max(1)
            };
            ('\u{FFFD}', len)
        }
    }
}

fn first_char(s: &str) -> (char, usize) {
    s.chars()
        .next()
        .map_or(('\u{FFFD}', 1), |c| (c, c.len_utf8()))
}

/// Does `bytes` start with a WTF-8 encoded surrogate code point (`U+D800`..=`U+DFFF`)?
fn is_wtf8_surrogate(bytes: &[u8]) -> bool {
    matches!(bytes, [0xED, b1, b2, ..] if (0xA0..=0xBF).contains(b1) && (0x80..=0xBF).contains(b2))
}

/// The suffix of `s` starting at encoded-byte offset `pos`, as an owned
/// `OsString`. `pos` must be on a code point boundary.
fn os_suffix(s: &OsStr, pos: usize) -> Result<OsString, Error> {
    let bytes = s.as_encoded_bytes();
    let tail = bytes.get(pos..).unwrap_or(&[]);
    if let Ok(text) = std::str::from_utf8(tail) {
        // The common case, and the only one that needs no platform knowledge.
        return Ok(OsString::from(text));
    }
    platform::os_from_tail(s, tail)
}

#[cfg(unix)]
mod platform {
    use super::{Error, OsStr, OsString};
    use std::os::unix::ffi::OsStringExt;

    pub(super) fn os_from_tail(_s: &OsStr, tail: &[u8]) -> Result<OsString, Error> {
        Ok(OsString::from_vec(tail.to_vec()))
    }
}

#[cfg(windows)]
mod platform {
    use super::{Error, OsStr, OsString};
    use std::os::windows::ffi::OsStringExt;

    pub(super) fn os_from_tail(_s: &OsStr, tail: &[u8]) -> Result<OsString, Error> {
        Ok(OsString::from_wide(&super::wtf8::to_wide(tail)))
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use super::{Error, OsStr, OsString};

    pub(super) fn os_from_tail(s: &OsStr, _tail: &[u8]) -> Result<OsString, Error> {
        Err(Error::NonUnicodeValue(s.to_os_string()))
    }
}

/// WTF-8 → UTF-16 conversion, needed to rebuild a Windows `OsString` from a
/// byte slice without `unsafe`. Compiled on every platform under `test` so the
/// logic is exercised by the regular test suite.
#[cfg(any(windows, test))]
mod wtf8 {
    /// Convert well-formed WTF-8 to UTF-16 code units. Ill-formed input maps
    /// each offending byte to `U+FFFD`; it cannot occur for strings that came
    /// out of a Windows `OsStr`.
    pub(crate) fn to_wide(bytes: &[u8]) -> Vec<u16> {
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while let Some(&b0) = bytes.get(i) {
            let (cp, len) = if b0 < 0x80 {
                (u32::from(b0), 1)
            } else if b0 & 0xE0 == 0xC0 {
                (u32::from(b0 & 0x1F) << 6 | cont(bytes, i + 1), 2)
            } else if b0 & 0xF0 == 0xE0 {
                (
                    u32::from(b0 & 0x0F) << 12 | cont(bytes, i + 1) << 6 | cont(bytes, i + 2),
                    3,
                )
            } else if b0 & 0xF8 == 0xF0 {
                (
                    u32::from(b0 & 0x07) << 18
                        | cont(bytes, i + 1) << 12
                        | cont(bytes, i + 2) << 6
                        | cont(bytes, i + 3),
                    4,
                )
            } else {
                (0xFFFD, 1)
            };
            let len = len.min(bytes.len() - i);
            if cp >= 0x1_0000 {
                let v = cp - 0x1_0000;
                out.push(0xD800 | u16::try_from(v >> 10).unwrap_or(0));
                out.push(0xDC00 | u16::try_from(v & 0x3FF).unwrap_or(0));
            } else {
                out.push(u16::try_from(cp).unwrap_or(0xFFFD));
            }
            i += len;
        }
        out
    }

    fn cont(bytes: &[u8], i: usize) -> u32 {
        u32::from(bytes.get(i).copied().unwrap_or(0) & 0x3F)
    }
}

/// Fallible conversions on [`OsString`] values returned by the parser.
///
/// The trait is sealed; it exists so that `parser.value()?.parse()?` reads
/// naturally while keeping the conversion explicit.
pub trait ValueExt: private::Sealed {
    /// Parse the value with [`FromStr`], after checking it is valid Unicode.
    ///
    /// # Errors
    ///
    /// [`Error::NonUnicodeValue`] or [`Error::ParsingFailed`].
    fn parse<T: FromStr>(&self) -> Result<T, Error>
    where
        T::Err: Into<Box<dyn StdError + Send + Sync + 'static>>;

    /// Parse the value with a custom function, after checking it is valid Unicode.
    ///
    /// # Errors
    ///
    /// [`Error::NonUnicodeValue`] or [`Error::ParsingFailed`].
    fn parse_with<F, T, E>(&self, f: F) -> Result<T, Error>
    where
        F: FnOnce(&str) -> Result<T, E>,
        E: Into<Box<dyn StdError + Send + Sync + 'static>>;

    /// Convert the value into a `String`.
    ///
    /// # Errors
    ///
    /// [`Error::NonUnicodeValue`] if the value is not valid Unicode.
    fn string(self) -> Result<String, Error>;
}

mod private {
    pub trait Sealed {}
    impl Sealed for std::ffi::OsString {}
}

impl ValueExt for OsString {
    fn parse<T: FromStr>(&self) -> Result<T, Error>
    where
        T::Err: Into<Box<dyn StdError + Send + Sync + 'static>>,
    {
        self.parse_with(T::from_str)
    }

    fn parse_with<F, T, E>(&self, f: F) -> Result<T, Error>
    where
        F: FnOnce(&str) -> Result<T, E>,
        E: Into<Box<dyn StdError + Send + Sync + 'static>>,
    {
        let text = self
            .to_str()
            .ok_or_else(|| Error::NonUnicodeValue(self.clone()))?;
        f(text).map_err(|e| Error::ParsingFailed {
            value: text.to_owned(),
            error: e.into(),
        })
    }

    fn string(self) -> Result<String, Error> {
        self.into_string().map_err(Error::NonUnicodeValue)
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn decode_char_handles_ascii_multibyte_and_garbage() {
        assert_eq!(decode_char(b"abc"), ('a', 1));
        assert_eq!(decode_char("éa".as_bytes()), ('é', 2));
        assert_eq!(decode_char("日本".as_bytes()), ('日', 3));
        assert_eq!(decode_char("😀".as_bytes()), ('😀', 4));
        assert_eq!(decode_char(b"\xffab"), ('\u{FFFD}', 1));
        // WTF-8 surrogate U+D800 = ED A0 80.
        assert_eq!(decode_char(b"\xED\xA0\x80x"), ('\u{FFFD}', 3));
        // Truncated sequence at the end.
        assert_eq!(decode_char(b"\xE3\x81"), ('\u{FFFD}', 2));
        // Valid-then-invalid.
        assert_eq!(decode_char(b"a\xff"), ('a', 1));
    }

    #[test]
    fn wtf8_to_wide_round_trips() {
        let s = "aé日😀";
        assert_eq!(
            wtf8::to_wide(s.as_bytes()),
            s.encode_utf16().collect::<Vec<_>>()
        );
        // Lone surrogate U+D83D.
        assert_eq!(wtf8::to_wide(b"\xED\xA0\xBD"), vec![0xD83D]);
        assert_eq!(
            wtf8::to_wide(b"x\xED\xA0\xBDy"),
            vec![b'x'.into(), 0xD83D, b'y'.into()]
        );
        // Garbage does not panic.
        assert_eq!(wtf8::to_wide(b"\xff"), vec![0xFFFD]);
        assert_eq!(wtf8::to_wide(b"\xE3\x81").len(), 1);
    }
}
