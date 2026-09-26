//! The result of parsing: [`Matches`].

use std::ffi::{OsStr, OsString};

use crate::arg::{AnyValue, Arg};

/// Where a value came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Source {
    /// Given on the command line.
    CommandLine,
    /// Read from an environment variable.
    Env,
    /// Filled in from the default.
    Default,
}

/// One stored occurrence of an argument.
pub(crate) struct Stored {
    pub(crate) raw: OsString,
    pub(crate) value: AnyValue,
}

pub(crate) struct Slot {
    pub(crate) id: String,
    pub(crate) values: Vec<Stored>,
    pub(crate) source: Option<Source>,
}

/// The parsed values of one command level. Subcommand matches are nested
/// through [`subcommand`](Matches::subcommand).
///
/// Values are read back with the same [`Arg`] that defined them, which
/// carries the type:
///
/// ```
/// use hasami::{Arg, Command};
///
/// let level = Arg::new("level").value::<u8>().default(3);
/// let cmd = Command::new("x").arg(&level);
/// let m = cmd.try_parse_args(["--level", "7"]).unwrap();
/// assert_eq!(m.get(&level), 7);
/// assert!(m.contains(&level));
/// ```
pub struct Matches {
    pub(crate) slots: Vec<Slot>,
    pub(crate) sub: Option<Box<(String, Matches)>>,
    /// An unknown subcommand accepted through
    /// [`Command::allow_external_subcommands`](crate::Command::allow_external_subcommands):
    /// its name and every argument after it, untouched.
    pub(crate) external: Option<(OsString, Vec<OsString>)>,
}

impl std::fmt::Debug for Matches {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Matches");
        for slot in &self.slots {
            if !slot.values.is_empty() {
                let raws: Vec<&OsStr> = slot.values.iter().map(|v| v.raw.as_os_str()).collect();
                s.field(&slot.id, &raws);
            }
        }
        if let Some(sub) = &self.sub {
            s.field("subcommand", &sub.0);
            s.field("subcommand_matches", &sub.1);
        }
        if let Some((name, args)) = &self.external {
            s.field("external_subcommand", &(name, args));
        }
        s.finish()
    }
}

impl Matches {
    pub(crate) fn new(ids: impl IntoIterator<Item = String>) -> Matches {
        Matches {
            slots: ids
                .into_iter()
                .map(|id| Slot {
                    id,
                    values: Vec::new(),
                    source: None,
                })
                .collect(),
            sub: None,
            external: None,
        }
    }

    fn slot(&self, id: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.id == id)
    }

    /// The typed value of `arg`.
    ///
    /// # Panics
    ///
    /// If `arg` does not belong to the command these matches were produced
    /// by. That is a programming error comparable to indexing a `Vec` out of
    /// bounds; use [`try_get`](Matches::try_get) to get `None` instead.
    pub fn get<T>(&self, arg: &Arg<T>) -> T {
        match self.try_get(arg) {
            Some(v) => v,
            None => panic!(
                "argument {:?} is not part of the command that produced these matches",
                arg.id()
            ),
        }
    }

    /// The typed value of `arg`, or `None` if `arg` is not part of the
    /// command that produced these matches.
    pub fn try_get<T>(&self, arg: &Arg<T>) -> Option<T> {
        let slot = self.slot(arg.id())?;
        (arg.extract)(&arg.def, &slot.values)
    }

    /// Was `arg` given explicitly (on the command line or through the
    /// environment)? Defaults do not count.
    pub fn contains<T>(&self, arg: &Arg<T>) -> bool {
        self.contains_id(arg.id())
    }

    /// Like [`contains`](Matches::contains), by id.
    pub fn contains_id(&self, id: &str) -> bool {
        self.slot(id)
            .is_some_and(|s| matches!(s.source, Some(Source::CommandLine | Source::Env)))
    }

    /// How many times `arg` occurred on the command line.
    pub fn occurrences<T>(&self, arg: &Arg<T>) -> usize {
        match self.slot(arg.id()) {
            Some(s) if s.source == Some(Source::CommandLine) => s.values.len(),
            _ => 0,
        }
    }

    /// The raw, unparsed values of `arg` in the order given.
    pub fn raw<T>(&self, arg: &Arg<T>) -> Vec<&OsStr> {
        self.raw_id(arg.id())
    }

    /// Like [`raw`](Matches::raw), by id.
    pub fn raw_id(&self, id: &str) -> Vec<&OsStr> {
        self.slot(id)
            .map(|s| s.values.iter().map(|v| v.raw.as_os_str()).collect())
            .unwrap_or_default()
    }

    /// Where the value of `arg` came from, if it has one.
    pub fn source<T>(&self, arg: &Arg<T>) -> Option<Source> {
        self.slot(arg.id()).and_then(|s| s.source)
    }

    /// The selected subcommand, as its name and its own matches.
    pub fn subcommand(&self) -> Option<(&str, &Matches)> {
        self.sub.as_deref().map(|(name, m)| (name.as_str(), m))
    }

    /// The selected subcommand's name.
    pub fn subcommand_name(&self) -> Option<&str> {
        self.sub.as_deref().map(|(name, _)| name.as_str())
    }

    /// The matches of the selected subcommand, if it is `name`.
    pub fn subcommand_matches(&self, name: &str) -> Option<&Matches> {
        match self.sub.as_deref() {
            Some((n, m)) if n == name => Some(m),
            _ => None,
        }
    }

    /// An external subcommand (one not defined on the command, accepted
    /// through [`Command::allow_external_subcommands`](crate::Command::allow_external_subcommands)):
    /// its name and the raw arguments that followed it.
    pub fn external_subcommand(&self) -> Option<(&OsStr, &[OsString])> {
        self.external
            .as_ref()
            .map(|(name, args)| (name.as_os_str(), args.as_slice()))
    }

    /// The ids of every argument that was given explicitly (on the command
    /// line or through the environment), in definition order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.slots
            .iter()
            .filter(|s| matches!(s.source, Some(Source::CommandLine | Source::Env)))
            .map(|s| s.id.as_str())
    }
}
