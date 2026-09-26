//! # hasami
//!
//! A layered command line argument parser.
//!
//! * [`lex`] (the `hasami-core` crate): a dependency-free, panic-free
//!   GNU/POSIX lexer you drive by hand. Use it when every byte and every
//!   millisecond of build time counts.
//! * The declarative builder in this crate: [`Command`], [`Arg`],
//!   [`Group`], [`Subcommand`]. Typed values, constraints, subcommands,
//!   help. Everything is a plain value; parsing never mutates the definition.
//! * The [`cli!`] macro: a `macro_rules!` DSL that turns a struct-like
//!   definition into a `Command` plus a typed struct, with no proc-macro.
//! * `#[derive(Args)]` and `#[derive(Commands)]` (feature `derive`): the
//!   same as `cli!`, as proc macros with `#[hasami(...)]` attributes.
//!
//! All front ends lower to the same [`Command`] value, so help, errors,
//! completions and schemas are identical whichever you use.
//!
//! ```
//! use hasami::{Arg, Command};
//!
//! let number = Arg::new("number").short('n').value::<u32>().default(1).help("How many times");
//! let shout = Arg::new("shout").help("Use upper case");
//! let thing = Arg::positional::<String>("THING").required().help("Whom to greet");
//!
//! let cmd = Command::new("greet")
//!     .about("Greet someone")
//!     .arg(&number)
//!     .arg(&shout)
//!     .arg(&thing);
//!
//! let m = cmd.try_parse_args(["-n", "2", "--shout", "world"]).unwrap();
//! assert_eq!(m.get(&number), 2);
//! assert!(m.get(&shout));
//! assert_eq!(m.get(&thing), "world");
//!
//! let err = cmd.try_parse_args(["--nubmer", "2", "world"]).unwrap_err();
//! assert!(err.to_string().starts_with("error: unexpected argument '--nubmer' found"));
//! ```
//!
//! ## Features
//!
//! | Feature    | Default | Adds |
//! |------------|---------|------|
//! | `help`     | yes     | `-h/--help`, `-V/--version`, help rendering |
//! | `suggest`  | no      | "a similar argument exists" tips |
//! | `color`    | no      | ANSI colour in help and errors |
//! | `env`      | no      | [`Arg::env`] fallback |
//! | `derive`   | no      | `#[derive(Args)]` |
//!
//! Shell completion, documentation and JSON schema generation are separate
//! crates that take a [`Command`]: `hasami-complete`, `hasami-doc`,
//! `hasami-schema`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// The lexer layer, re-exported from `hasami-core`.
pub use hasami_core as lex;

#[doc(hidden)]
#[path = "macro_support.rs"]
pub mod __macro;
mod arg;
mod command;
mod error;
#[cfg(feature = "help")]
mod help;
mod macros;
mod matches;
mod parse;
mod style;
#[cfg(feature = "suggest")]
mod suggest;
mod traits;

pub use arg::{Arg, ArgDef, Completer, Relation, ValueEnum};
pub use command::{Command, Group, Subcommand};
pub use error::{Error, ErrorKind};
pub use matches::{Matches, Source};
pub use style::{Stream, Styles};
pub use traits::{Cli, Subcommands};

#[cfg(feature = "derive")]
pub use hasami_derive::{Args, Commands, ValueEnum};

/// Convenience re-exports: the traits, the builder types and the derives.
pub mod prelude {
    // `ValueEnum` is both the trait and, with `derive`, the derive macro.
    pub use crate::{Arg, Cli, Command, Subcommands, ValueEnum};
    #[cfg(feature = "derive")]
    pub use hasami_derive::{Args, Commands};
}
