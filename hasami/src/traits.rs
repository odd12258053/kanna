//! The traits implemented by [`cli!`](crate::cli) and `#[derive(Args)]`
//! definitions.

use std::ffi::OsString;

use crate::{Command, Error, Matches, Subcommand};

/// A struct that describes a command line and can be built from parsed
/// matches. Implemented by [`cli!`](crate::cli) and by `#[derive(Args)]`.
///
/// Only [`command`](Cli::command), [`from_matches`](Cli::from_matches) and
/// [`ABOUT`](Cli::ABOUT) are required; the parsing entry points are provided.
pub trait Cli: Sized {
    /// The doc comment of the definition, raw (lines keep their leading
    /// space), or `None` if there was none.
    const ABOUT: Option<&'static str>;

    /// The [`Command`] this definition lowers to.
    fn command() -> Command;

    /// Build the struct from parsed matches of [`command`](Cli::command).
    fn from_matches(matches: &Matches) -> Result<Self, Error>;

    /// Parse the process arguments, exiting on error (status 0 for
    /// `--help`/`--version`, 2 otherwise).
    fn parse() -> Self {
        Self::try_parse().unwrap_or_else(|e| e.exit())
    }

    /// Parse the process arguments.
    fn try_parse() -> Result<Self, Error> {
        Self::try_parse_from(std::env::args_os())
    }

    /// Parse `args` (first item is the binary name), exiting on error.
    fn parse_from<I>(args: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        Self::try_parse_from(args).unwrap_or_else(|e| e.exit())
    }

    /// Parse `args`, whose first item is the binary name.
    fn try_parse_from<I>(args: I) -> Result<Self, Error>
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        let m = Self::command().try_parse_from(args)?;
        Self::from_matches(&m)
    }

    /// Parse `args`, which contain no binary name.
    fn try_parse_args<I>(args: I) -> Result<Self, Error>
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        let m = Self::command().try_parse_args(args)?;
        Self::from_matches(&m)
    }
}

/// An enum whose variants are subcommands. Implemented by
/// [`cli!`](crate::cli) and by `#[derive(Commands)]`.
pub trait Subcommands: Sized {
    /// The subcommand entries, one per variant.
    fn subcommands() -> Vec<Subcommand>;

    /// Build the enum from the selected subcommand in `matches`, if any.
    fn from_matches(matches: &Matches) -> Result<Option<Self>, Error>;
}
