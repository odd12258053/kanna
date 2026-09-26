//! The error type returned by parsing.

use std::error::Error as StdError;
use std::fmt;

use crate::style::Styles;

/// What went wrong. `#[non_exhaustive]`: match with a wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An option that is not defined.
    UnknownOption,
    /// A subcommand that is not defined.
    UnknownSubcommand,
    /// A positional argument with no slot to go in.
    UnexpectedArgument,
    /// An option that needs a value was given none.
    MissingValue,
    /// An option that takes no value was given one (`--flag=x`).
    UnexpectedValue,
    /// A value failed validation or parsing.
    InvalidValue,
    /// A required argument or group was not provided.
    MissingRequired,
    /// A subcommand was required but not given.
    MissingSubcommand,
    /// Arguments that cannot be used together were.
    Conflict,
    /// A single-value argument or a flag was given more than once.
    Repeated,
    /// No arguments were given to a command that asked for
    /// [`arg_required_else_help`](crate::Command::arg_required_else_help);
    /// the message is the help text, exit status 2.
    HelpOnMissingArgs,
    /// An argument was not valid Unicode where Unicode was needed.
    NonUnicode,
    /// `--help` was requested; the message is the help text.
    DisplayHelp,
    /// `--version` was requested; the message is the version line.
    DisplayVersion,
    /// An I/O error while printing.
    Io,
    /// An error produced by application code.
    Custom,
}

/// A parse error, or a request to display help or the version.
///
/// `Display` renders the full user-facing message. Use
/// [`exit`](Error::exit) to print it to the right stream and exit with the
/// conventional status: 0 for help/version, 2 for usage errors.
pub struct Error {
    kind: ErrorKind,
    message: String,
    /// The less common fields, boxed so that `Result<T, Error>` stays
    /// small (clippy's `result_large_err` threshold is 128 bytes).
    details: Option<Box<Details>>,
    /// The command the error belongs to, so help and usage can be
    /// re-rendered with colour at print time.
    #[cfg(feature = "color")]
    help: Option<Box<HelpContext>>,
}

/// Optional parts of an [`Error`].
#[derive(Default)]
struct Details {
    tip: Option<String>,
    usage: Option<String>,
    /// The id of the argument the error is about, when known.
    arg: Option<String>,
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

#[cfg(feature = "color")]
pub(crate) struct HelpContext {
    pub(crate) cmd: crate::Command,
    pub(crate) path: String,
    /// Render the `--help` (long) form rather than the `-h` form.
    pub(crate) long: bool,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("kind", &self.kind)
            .field("message", &self.message)
            .field("tip", &self.tip())
            .finish_non_exhaustive()
    }
}

impl Error {
    /// A new error of the given kind with a message. No usage line is
    /// attached; the parser adds one for errors it produces.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Error {
        Error {
            kind,
            message: message.into(),
            details: None,
            #[cfg(feature = "color")]
            help: None,
        }
    }

    /// An application error, exit status 2, rendered as `error: {message}`.
    pub fn custom(message: impl Into<String>) -> Error {
        Error::new(ErrorKind::Custom, message)
    }

    /// Attach a "tip:" line telling the user how to fix the problem.
    pub fn with_tip(mut self, tip: impl Into<String>) -> Error {
        self.details_mut().tip = Some(tip.into());
        self
    }

    /// Attach a usage line, shown after the message.
    pub fn with_usage(mut self, usage: impl Into<String>) -> Error {
        self.details_mut().usage = Some(usage.into());
        self
    }

    /// Record which argument (by id) the error is about. The parser sets
    /// this for errors that concern one argument; it is reported by
    /// [`arg`](Error::arg) and in [`to_json`](Error::to_json).
    pub fn with_arg(mut self, id: impl Into<String>) -> Error {
        self.details_mut().arg = Some(id.into());
        self
    }

    /// Attach an underlying error.
    pub fn with_source(mut self, source: impl StdError + Send + Sync + 'static) -> Error {
        self.details_mut().source = Some(Box::new(source));
        self
    }

    fn details_mut(&mut self) -> &mut Details {
        self.details.get_or_insert_with(Box::default)
    }

    pub(crate) fn set_usage_if_missing(&mut self, usage: impl FnOnce() -> String) {
        if self.usage().is_none() && self.kind.is_usage_error() {
            self.details_mut().usage = Some(usage());
        }
    }

    /// Remember which command the error is about, for coloured output.
    #[cfg(feature = "color")]
    pub(crate) fn with_help_context(
        mut self,
        cmd: &crate::Command,
        path: &str,
        long: bool,
    ) -> Error {
        if self.help.is_none()
            && (self.kind == ErrorKind::DisplayHelp
                || self.kind == ErrorKind::HelpOnMissingArgs
                || self.kind.is_usage_error())
        {
            self.help = Some(Box::new(HelpContext {
                cmd: cmd.clone(),
                path: path.to_owned(),
                long,
            }));
        }
        self
    }

    /// Without the `color` feature nothing needs to be remembered.
    #[cfg(not(feature = "color"))]
    pub(crate) fn with_help_context(
        self,
        _cmd: &crate::Command,
        _path: &str,
        _long: bool,
    ) -> Error {
        self
    }

    /// The palette to render with: the command's own when colour is on for
    /// the stream, plain otherwise.
    fn styles_for(&self, stream: crate::Stream) -> Styles {
        let st = Styles::for_stream(stream);
        #[cfg(feature = "color")]
        if !st.is_plain() {
            if let Some(h) = &self.help {
                return h.cmd.styles;
            }
        }
        st
    }

    /// The kind of error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
    /// The bare message, without `error:` prefix, tip or usage.
    pub fn message(&self) -> &str {
        &self.message
    }
    /// The tip, if any.
    pub fn tip(&self) -> Option<&str> {
        self.details.as_ref()?.tip.as_deref()
    }
    /// The usage line, if any.
    pub fn usage(&self) -> Option<&str> {
        self.details.as_ref()?.usage.as_deref()
    }
    /// The id of the argument the error is about, if known.
    pub fn arg(&self) -> Option<&str> {
        self.details.as_ref()?.arg.as_deref()
    }

    /// The error as one JSON object (feature `json`), for programs and
    /// agents that read errors rather than people:
    ///
    /// ```text
    /// {"kind":"invalid_value","exit_code":2,"arg":"number",
    ///  "message":"invalid value 'x' for '--number <NUMBER>': ...",
    ///  "tip":null,"usage":"app [OPTIONS]"}
    /// ```
    ///
    /// `kind` is [`ErrorKind::name`]. For help and version requests
    /// `message` holds the text that would have been printed.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\"kind\":");
        json_str(&mut s, self.kind.name());
        s.push_str(",\"exit_code\":");
        s.push_str(if self.is_display() { "0" } else { "2" });
        for (key, value) in [
            ("arg", self.arg()),
            ("message", Some(self.message.as_str())),
            ("tip", self.tip()),
            ("usage", self.usage()),
        ] {
            s.push_str(",\"");
            s.push_str(key);
            s.push_str("\":");
            match value {
                Some(v) => json_str(&mut s, v),
                None => s.push_str("null"),
            }
        }
        s.push('}');
        s
    }

    /// `true` for help and version requests, which are not failures.
    pub fn is_display(&self) -> bool {
        matches!(
            self.kind,
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
        )
    }

    /// The conventional process exit status: 0 for help/version, 2 otherwise.
    pub fn exit_code(&self) -> i32 {
        if self.is_display() { 0 } else { 2 }
    }

    /// Print the rendered message: to stdout for help/version, to stderr
    /// otherwise. With the `color` feature, ANSI colour is used when the
    /// stream is a terminal (see [`Styles`](crate::Styles)).
    pub fn print(&self) -> std::io::Result<()> {
        use std::io::Write;
        #[cfg(feature = "json")]
        if !self.is_display()
            && std::env::var_os("HASAMI_ERROR_FORMAT").is_some_and(|v| v == "json")
        {
            let err = std::io::stderr();
            let mut err = err.lock();
            err.write_all(self.to_json().as_bytes())?;
            err.write_all(b"\n")?;
            return err.flush();
        }
        if self.is_display() {
            let text = self.render(&self.styles_for(crate::Stream::Stdout));
            let out = std::io::stdout();
            let mut out = out.lock();
            out.write_all(text.as_bytes())?;
            // Help already ends with a newline; the version line does not.
            if !text.ends_with('\n') {
                out.write_all(b"\n")?;
            }
            out.flush()
        } else {
            let text = self.render(&self.styles_for(crate::Stream::Stderr));
            let err = std::io::stderr();
            let mut err = err.lock();
            err.write_all(text.as_bytes())?;
            err.write_all(b"\n")?;
            err.flush()
        }
    }

    /// The message rendered with the given styles. `Display` uses
    /// [`Styles::PLAIN`].
    pub fn render(&self, st: &Styles) -> String {
        if self.is_display() || self.kind == ErrorKind::HelpOnMissingArgs {
            #[cfg(feature = "color")]
            if let Some(h) = &self.help {
                if self.kind != ErrorKind::DisplayVersion {
                    return crate::help::render_help_styled(&h.cmd, &h.path, st, h.long);
                }
            }
            return self.message.clone();
        }
        let mut s = String::new();
        s.push_str(st.error());
        s.push_str("error:");
        s.push_str(st.reset());
        s.push(' ');
        s.push_str(&self.message);
        if let Some(tip) = self.tip() {
            s.push_str("\n\n  ");
            s.push_str(st.tip());
            s.push_str("tip:");
            s.push_str(st.reset());
            s.push(' ');
            s.push_str(tip);
        }
        if let Some(usage) = self.usage() {
            s.push_str("\n\n");
            s.push_str(st.header());
            s.push_str("Usage:");
            s.push_str(st.reset());
            s.push(' ');
            #[cfg(feature = "color")]
            let usage = &self
                .help
                .as_ref()
                .map(|h| crate::help::render_usage_styled(&h.cmd, &h.path, st))
                .unwrap_or_else(|| usage.to_owned());
            s.push_str(usage);
            if cfg!(feature = "help") {
                s.push_str("\n\nFor more information, try '");
                s.push_str(st.literal());
                s.push_str("--help");
                s.push_str(st.reset());
                s.push_str("'.");
            }
        }
        s
    }

    /// Print the message and exit the process with
    /// [`exit_code`](Error::exit_code).
    pub fn exit(&self) -> ! {
        // A failed write to a closed pipe is not worth a second error.
        let _ = self.print();
        std::process::exit(self.exit_code())
    }
}

impl ErrorKind {
    /// The kind as a stable `snake_case` name (`unknown_option`,
    /// `invalid_value`, ...), used by [`Error::to_json`].
    pub fn name(self) -> &'static str {
        match self {
            ErrorKind::UnknownOption => "unknown_option",
            ErrorKind::UnknownSubcommand => "unknown_subcommand",
            ErrorKind::UnexpectedArgument => "unexpected_argument",
            ErrorKind::MissingValue => "missing_value",
            ErrorKind::UnexpectedValue => "unexpected_value",
            ErrorKind::InvalidValue => "invalid_value",
            ErrorKind::MissingRequired => "missing_required",
            ErrorKind::MissingSubcommand => "missing_subcommand",
            ErrorKind::Conflict => "conflict",
            ErrorKind::Repeated => "repeated",
            ErrorKind::HelpOnMissingArgs => "help_on_missing_args",
            ErrorKind::NonUnicode => "non_unicode",
            ErrorKind::DisplayHelp => "display_help",
            ErrorKind::DisplayVersion => "display_version",
            ErrorKind::Io => "io",
            ErrorKind::Custom => "custom",
        }
    }

    fn is_usage_error(self) -> bool {
        !matches!(
            self,
            ErrorKind::DisplayHelp
                | ErrorKind::DisplayVersion
                | ErrorKind::HelpOnMissingArgs
                | ErrorKind::Io
                | ErrorKind::Custom
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(&Styles::PLAIN))
    }
}

/// Append `s` as a JSON string literal.
#[cfg(feature = "json")]
fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.details
            .as_ref()?
            .source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn StdError + 'static))
    }
}

impl From<hasami_core::Error> for Error {
    fn from(e: hasami_core::Error) -> Error {
        use hasami_core::Error as Core;
        let kind = match &e {
            Core::MissingValue { .. } => ErrorKind::MissingValue,
            Core::UnexpectedOption(_) => ErrorKind::UnknownOption,
            Core::UnexpectedArgument(_) => ErrorKind::UnexpectedArgument,
            Core::UnexpectedValue { .. } => ErrorKind::UnexpectedValue,
            Core::NonUnicodeValue(_) => ErrorKind::NonUnicode,
            Core::ParsingFailed { .. } => ErrorKind::InvalidValue,
            _ => ErrorKind::Custom,
        };
        // The core error is fully rendered in the message; boxing it as a
        // source would also link its `Debug` impl for a chain nobody walks.
        Error::new(kind, e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::new(ErrorKind::Io, e.to_string()).with_source(e)
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Error {
        Error::custom(msg)
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Error {
        Error::custom(msg)
    }
}
