//! A read-eval-print loop (REPL) for a [`kanna::Command`]: the program
//! reads one line at a time, splits it like a shell, parses it with the
//! same definition that parses the command line, and hands the result to
//! a handler. Help, errors, suggestions and value checking come from the
//! definition, so a REPL costs one subcommand and a handler.
//!
//! ```no_run
//! use kanna::{Arg, Command};
//! use kanna_prompt::{Flow, Repl};
//!
//! let name = Arg::positional::<String>("NAME").required().help("Who");
//! let cmd = Command::new("app")
//!     .subcommand(Command::new("greet").about("Say hello").arg(&name))
//!     .subcommand(Command::new("stats").about("Show numbers"));
//! Repl::new(&cmd).prompt("app> ").run(|m| {
//!     match m.subcommand() {
//!         Some(("greet", sm)) => println!("hello, {}", sm.raw_id("NAME")[0].to_string_lossy()),
//!         Some(("stats", _)) => println!("nothing yet"),
//!         _ => {}
//!     }
//!     Ok(Flow::Continue)
//! }).unwrap();
//! ```
//!
//! Each line is an argument vector **without** the binary name, so
//! `greet bob` at the prompt is `app greet bob` on the command line.
//! A few words are handled before parsing: an empty line or one that
//! starts with `#` is skipped; `exit` and `quit` end the loop (unless the
//! command defines a subcommand of that name); `help` is `--help`. Help
//! and version requests print to standard output; parse errors print to
//! standard error the way [`kanna::Error::print`] does (colour, and JSON
//! with `KANNA_ERROR_FORMAT=json`) and the loop continues. End of input
//! ends the loop, so a file of commands can be piped in; the prompt is
//! only shown when standard input is a terminal.
//!
//! No line editing or history: this crate has no dependencies. To add
//! them, drive [`Repl::run_line`] from a line-editing library and, with
//! the `complete` feature, feed [`complete_line`] to its completer.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::ffi::OsString;
use std::io::{self, BufRead, IsTerminal, Write};

use kanna::{Cli, Command, Error, Matches, Styles};

/// What the loop does after a handler returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// Read the next line.
    Continue,
    /// End the loop.
    Exit,
}

/// One line of the loop: parsed and handled, or skipped, or the end.
#[derive(Debug, PartialEq, Eq)]
enum Step {
    Skipped,
    Handled(Flow),
}

/// The loop: a command definition, a prompt, and the words that end it.
pub struct Repl<'a> {
    cmd: &'a Command,
    prompt: String,
    exit_words: Vec<String>,
}

impl<'a> Repl<'a> {
    /// A loop over `cmd` with the prompt `> ` and the exit words `exit`
    /// and `quit`.
    pub fn new(cmd: &'a Command) -> Repl<'a> {
        Repl {
            cmd,
            prompt: "> ".to_owned(),
            exit_words: vec!["exit".to_owned(), "quit".to_owned()],
        }
    }

    /// The text shown before each line when standard input is a terminal.
    pub fn prompt(mut self, text: impl Into<String>) -> Repl<'a> {
        self.prompt = text.into();
        self
    }

    /// The words that end the loop when typed alone; replaces `exit` and
    /// `quit`. A subcommand of the same name takes precedence.
    pub fn exit_words<I, S>(mut self, words: I) -> Repl<'a>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.exit_words = words.into_iter().map(Into::into).collect();
        self
    }

    /// Run on standard input and output until end of input, an exit word,
    /// or a handler returning [`Flow::Exit`]. Errors are printed with
    /// [`Error::print`] and do not end the loop.
    pub fn run<F>(&self, mut handler: F) -> io::Result<()>
    where
        F: FnMut(&Matches) -> Result<Flow, Error>,
    {
        let stdin = io::stdin();
        let show_prompt = stdin.is_terminal();
        let mut lines = stdin.lock().lines();
        let mut out = io::stdout();
        loop {
            if show_prompt {
                out.write_all(self.prompt.as_bytes())?;
                out.flush()?;
            }
            let Some(line) = lines.next() else {
                if show_prompt {
                    out.write_all(b"\n")?;
                }
                return Ok(());
            };
            let line = line?;
            let step = self.step(&line, &mut handler, |e| e.print())?;
            if step == Step::Handled(Flow::Exit) {
                return Ok(());
            }
        }
    }

    /// Like [`run`](Repl::run) with typed arguments: each line is parsed
    /// into `A` (a `cli!` or `#[derive(Args)]` struct whose
    /// [`command`](Cli::command) is the loop's command).
    pub fn run_typed<A, F>(&self, mut handler: F) -> io::Result<()>
    where
        A: Cli,
        F: FnMut(A) -> Result<Flow, Error>,
    {
        self.run(|m| handler(A::from_matches(m)?))
    }

    /// Run on the given reader and writers instead of the standard
    /// streams, with the prompt always shown and messages rendered without
    /// colour. Help and version go to `out`, errors to `err`.
    pub fn run_with<R, W, E, F>(
        &self,
        input: R,
        mut out: W,
        mut err: E,
        mut handler: F,
    ) -> io::Result<()>
    where
        R: BufRead,
        W: Write,
        E: Write,
        F: FnMut(&Matches) -> Result<Flow, Error>,
    {
        for line in input.lines() {
            out.write_all(self.prompt.as_bytes())?;
            out.flush()?;
            let line = line?;
            let step = self.step(&line, &mut handler, |e| {
                let text = e.render(&Styles::PLAIN);
                if e.is_display() {
                    out.write_all(text.as_bytes())?;
                    if !text.ends_with('\n') {
                        out.write_all(b"\n")?;
                    }
                    out.flush()
                } else {
                    err.write_all(text.as_bytes())?;
                    err.write_all(b"\n")?;
                    err.flush()
                }
            })?;
            if step == Step::Handled(Flow::Exit) {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Handle one line: split it, parse it, call `handler`. Returns the
    /// handler's [`Flow`], or `None` when the line was skipped (empty or a
    /// `#` comment). An exit word yields `Some(Flow::Exit)`. Errors, both
    /// from parsing and from the handler, are returned rather than printed,
    /// so a line-editing front end can show them as it likes.
    ///
    /// # Errors
    ///
    /// A parse error, a help or version request (`e.is_display()`), or the
    /// handler's error.
    pub fn run_line<F>(&self, line: &str, handler: F) -> Result<Option<Flow>, Error>
    where
        F: FnOnce(&Matches) -> Result<Flow, Error>,
    {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return Ok(None);
        }
        let mut words = kanna::split_words(line);
        if let [only] = words.as_slice() {
            if self.exit_words.iter().any(|w| w == only) && self.cmd.find_subcommand(only).is_none()
            {
                return Ok(Some(Flow::Exit));
            }
            if only == "help" && self.cmd.find_subcommand("help").is_none() {
                words[0] = "--help".to_owned();
            }
        }
        let argv: Vec<OsString> = words.into_iter().map(OsString::from).collect();
        let m = self.cmd.try_parse_args(argv)?;
        handler(&m).map(Some)
    }

    /// `run_line`, reporting errors through `report` instead of returning
    /// them.
    fn step<F, P>(&self, line: &str, handler: &mut F, report: P) -> io::Result<Step>
    where
        F: FnMut(&Matches) -> Result<Flow, Error>,
        P: FnOnce(&Error) -> io::Result<()>,
    {
        match self.run_line(line, |m| handler(m)) {
            Ok(None) => Ok(Step::Skipped),
            Ok(Some(flow)) => Ok(Step::Handled(flow)),
            Err(e) => {
                report(&e)?;
                Ok(Step::Handled(Flow::Continue))
            }
        }
    }
}

/// Completion candidates for a partly typed `line`: the word being typed
/// is the last one, or a new empty word when `line` ends with whitespace.
/// Feed this to a line-editing library's completer.
///
/// ```
/// use kanna::{Arg, Command};
/// let cmd = Command::new("app")
///     .subcommand(Command::new("greet"))
///     .subcommand(Command::new("goodbye"));
/// let values: Vec<String> = kanna_prompt::complete_line(&cmd, "g")
///     .into_iter()
///     .map(|c| c.value)
///     .collect();
/// assert_eq!(values, ["greet", "goodbye"]);
/// ```
#[cfg(feature = "complete")]
pub fn complete_line(cmd: &Command, line: &str) -> Vec<kanna_complete::dynamic::Candidate> {
    let mut words = vec![cmd.name().to_owned()];
    words.extend(kanna::split_words(line));
    if line.is_empty() || line.ends_with(char::is_whitespace) {
        words.push(String::new());
    }
    let index = words.len() - 1;
    kanna_complete::dynamic::complete(cmd, &words, index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanna::{Arg, ErrorKind};
    use std::cell::RefCell;

    fn app() -> Command {
        let name = Arg::positional::<String>("NAME").required().help("Who");
        let loud = Arg::new("loud").short('l').help("Shout");
        Command::new("app")
            .version("1.0")
            .about("A test app")
            .subcommand(
                Command::new("greet")
                    .about("Say hello")
                    .arg(&name)
                    .arg(&loud),
            )
            .subcommand(Command::new("quit").about("A subcommand called quit"))
    }

    fn drive(repl: &Repl<'_>, input: &str) -> (Vec<String>, String, String) {
        let seen = RefCell::new(Vec::new());
        let mut out = Vec::new();
        let mut err = Vec::new();
        repl.run_with(input.as_bytes(), &mut out, &mut err, |m| {
            let (name, sm) = m.subcommand().unwrap();
            let mut s = name.to_owned();
            if name == "greet" {
                s.push(' ');
                s.push_str(&sm.raw_id("NAME")[0].to_string_lossy());
                if sm.contains_id("loud") {
                    s.push('!');
                }
            }
            seen.borrow_mut().push(s.clone());
            Ok(if s == "greet bye" {
                Flow::Exit
            } else {
                Flow::Continue
            })
        })
        .unwrap();
        (
            seen.into_inner(),
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn lines_are_parsed_and_handled() {
        let cmd = app();
        let repl = Repl::new(&cmd).prompt("app> ");
        let (seen, out, err) = drive(&repl, "greet bob\n\n# a comment\ngreet -l 'Ann Lee'\n");
        assert_eq!(seen, ["greet bob", "greet Ann Lee!"]);
        assert_eq!(out, "app> app> app> app> ");
        assert_eq!(err, "");
    }

    #[test]
    fn errors_are_reported_and_the_loop_continues() {
        let cmd = app();
        let repl = Repl::new(&cmd);
        let (seen, _, err) = drive(&repl, "greet\ngreeet bob\ngreet bob\n");
        assert_eq!(seen, ["greet bob"]);
        assert!(err.starts_with("error: the following required arguments were not provided:\n  <NAME>\n\nUsage: app greet"), "{err}");
        assert!(
            err.contains("error: unrecognized subcommand 'greeet'"),
            "{err}"
        );
    }

    #[test]
    fn help_version_and_exit_words() {
        let cmd = app();
        let repl = Repl::new(&cmd);
        let (seen, out, err) = drive(&repl, "help\n--version\nexit\ngreet never\n");
        assert_eq!(seen, Vec::<String>::new());
        assert!(
            out.contains("A test app\n\nUsage: app [OPTIONS] [COMMAND]"),
            "{out}"
        );
        assert!(out.contains("app 1.0\n"), "{out}");
        assert_eq!(err, "");
        // `quit` is a subcommand here, so it is not an exit word.
        let (seen, ..) = drive(&repl, "quit\ngreet bob\n");
        assert_eq!(seen, ["quit", "greet bob"]);
        let custom = Repl::new(&cmd).exit_words(["bye"]);
        let (seen, ..) = drive(&custom, "bye\ngreet bob\n");
        assert_eq!(seen, Vec::<String>::new());
    }

    #[test]
    fn handler_can_end_the_loop() {
        let cmd = app();
        let repl = Repl::new(&cmd);
        let (seen, ..) = drive(&repl, "greet bye\ngreet bob\n");
        assert_eq!(seen, ["greet bye"]);
    }

    #[test]
    fn run_line_returns_instead_of_printing() {
        let cmd = app();
        let repl = Repl::new(&cmd);
        assert_eq!(repl.run_line("   ", |_| Ok(Flow::Continue)).unwrap(), None);
        assert_eq!(
            repl.run_line("exit", |_| Ok(Flow::Continue)).unwrap(),
            Some(Flow::Exit)
        );
        let e = repl.run_line("greet", |_| Ok(Flow::Continue)).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::MissingRequired);
        let e = repl.run_line("help", |_| Ok(Flow::Continue)).unwrap_err();
        assert!(e.is_display());
        let e = repl
            .run_line("greet bob", |_| Err(Error::custom("no")))
            .unwrap_err();
        assert_eq!(e.message(), "no");
    }

    #[test]
    fn typed_run_builds_the_struct() {
        kanna::cli! {
            /// Typed
            #[name = "typed"]
            struct Args {
                #[subcommand] cmd: Cmd,
            }
            enum Cmd {
                /// Add
                Add(Add),
                /// Bye
                Bye,
            }
            struct Add {
                #[positional] n: u32,
            }
        }
        // `run_typed` reads the standard streams; exercise the same path
        // through `run_line` and `from_matches`.
        let cmd = Args::command();
        let repl = Repl::new(&cmd);
        let total = RefCell::new(0);
        for line in ["add 2", "add 3", "bye"] {
            let flow = repl
                .run_line(line, |m| {
                    Ok(match Args::from_matches(m)?.cmd {
                        Cmd::Add(a) => {
                            *total.borrow_mut() += a.n;
                            Flow::Continue
                        }
                        Cmd::Bye => Flow::Exit,
                    })
                })
                .unwrap();
            if flow == Some(Flow::Exit) {
                break;
            }
        }
        assert_eq!(total.into_inner(), 5);
    }

    #[cfg(feature = "complete")]
    #[test]
    fn completion_for_a_partial_line() {
        let cmd = app();
        let values = |line: &str| -> Vec<String> {
            complete_line(&cmd, line)
                .into_iter()
                .map(|c| c.value)
                .collect()
        };
        assert_eq!(values("gr"), ["greet"]);
        assert_eq!(values("greet bob -"), ["-l", "--loud", "-h", "--help"]);
        assert_eq!(values(""), ["greet", "quit"]);
    }
}
