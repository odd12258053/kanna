//! ANSI colour for help and error output (feature `color`).
//!
//! Colour is decided once per stream from the environment, following the
//! conventions most tools agree on:
//!
//! * `NO_COLOR` set (to anything) disables colour;
//! * `CLICOLOR_FORCE` set (and not `0`) forces colour even for pipes;
//! * `CLICOLOR=0` disables colour;
//! * `TERM=dumb` disables colour;
//! * otherwise colour is used only when the stream is a terminal.

/// The stream a message is destined for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    /// Standard output (help, version).
    Stdout,
    /// Standard error (errors).
    Stderr,
}

/// A set of ANSI escape sequences, or all-empty strings when colour is off.
///
/// Without the `color` feature every accessor is a constant empty string,
/// so the styling code in help and error rendering compiles away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Styles {
    #[cfg(feature = "color")]
    inner: Inner,
}

#[cfg(feature = "color")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Inner {
    header: &'static str,
    literal: &'static str,
    placeholder: &'static str,
    error: &'static str,
    tip: &'static str,
    invalid: &'static str,
    reset: &'static str,
}

macro_rules! accessor {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$doc])*
            #[inline(always)]
            pub fn $name(&self) -> &'static str {
                #[cfg(feature = "color")]
                {
                    self.inner.$name
                }
                #[cfg(not(feature = "color"))]
                {
                    ""
                }
            }
        )*
    };
}

impl Styles {
    /// The palette a [`Command`](crate::Command) starts with:
    /// [`COLORED`](Styles::COLORED) with the feature, [`PLAIN`](Styles::PLAIN)
    /// without.
    pub const fn default_palette() -> Styles {
        #[cfg(feature = "color")]
        {
            Styles::COLORED
        }
        #[cfg(not(feature = "color"))]
        {
            Styles::PLAIN
        }
    }

    /// No colour at all.
    pub const PLAIN: Styles = Styles {
        #[cfg(feature = "color")]
        inner: Inner {
            header: "",
            literal: "",
            placeholder: "",
            error: "",
            tip: "",
            invalid: "",
            reset: "",
        },
    };

    /// The default palette: bold underlined headers, bold literals, cyan
    /// placeholders, bold red errors, green tips, yellow invalid input.
    #[cfg(feature = "color")]
    pub const COLORED: Styles = Styles {
        inner: Inner {
            header: "\x1b[1;4m",
            literal: "\x1b[1m",
            placeholder: "\x1b[36m",
            error: "\x1b[1;31m",
            tip: "\x1b[32m",
            invalid: "\x1b[33m",
            reset: "\x1b[0m",
        },
    };

    accessor! {
        /// Section headers (`Usage:`, `Options:`).
        header,
        /// Literal text the user types: option names, subcommand names.
        literal,
        /// Value placeholders such as `<NAME>`.
        placeholder,
        /// The `error:` word.
        error,
        /// The `tip:` word.
        tip,
        /// Invalid input echoed back to the user.
        invalid,
        /// End of any styled span.
        reset,
    }

    /// Replace the escape sequence for section headers. Without the `color`
    /// feature the call is accepted and ignored.
    #[allow(unused_mut, unused_variables)]
    pub const fn with_header(mut self, seq: &'static str) -> Styles {
        #[cfg(feature = "color")]
        {
            self.inner.header = seq;
        }
        self
    }
    /// Replace the escape sequence for literals (option and subcommand names). Without the `color`
    /// feature the call is accepted and ignored.
    #[allow(unused_mut, unused_variables)]
    pub const fn with_literal(mut self, seq: &'static str) -> Styles {
        #[cfg(feature = "color")]
        {
            self.inner.literal = seq;
        }
        self
    }
    /// Replace the escape sequence for value placeholders. Without the `color`
    /// feature the call is accepted and ignored.
    #[allow(unused_mut, unused_variables)]
    pub const fn with_placeholder(mut self, seq: &'static str) -> Styles {
        #[cfg(feature = "color")]
        {
            self.inner.placeholder = seq;
        }
        self
    }
    /// Replace the escape sequence for the `error:` word. Without the `color`
    /// feature the call is accepted and ignored.
    #[allow(unused_mut, unused_variables)]
    pub const fn with_error(mut self, seq: &'static str) -> Styles {
        #[cfg(feature = "color")]
        {
            self.inner.error = seq;
        }
        self
    }
    /// Replace the escape sequence for the `tip:` word. Without the `color`
    /// feature the call is accepted and ignored.
    #[allow(unused_mut, unused_variables)]
    pub const fn with_tip(mut self, seq: &'static str) -> Styles {
        #[cfg(feature = "color")]
        {
            self.inner.tip = seq;
        }
        self
    }
    /// Replace the escape sequence for invalid input echoed back. Without the `color`
    /// feature the call is accepted and ignored.
    #[allow(unused_mut, unused_variables)]
    pub const fn with_invalid(mut self, seq: &'static str) -> Styles {
        #[cfg(feature = "color")]
        {
            self.inner.invalid = seq;
        }
        self
    }
    /// `true` when every accessor is empty.
    #[inline(always)]
    pub fn is_plain(&self) -> bool {
        #[cfg(feature = "color")]
        {
            self.inner.reset.is_empty()
        }
        #[cfg(not(feature = "color"))]
        {
            true
        }
    }

    /// The palette to use for `stream`, per the environment rules in the
    /// module documentation. Always [`PLAIN`](Styles::PLAIN) without the
    /// `color` feature.
    pub fn for_stream(stream: Stream) -> Styles {
        #[cfg(feature = "color")]
        {
            if Self::wanted(stream) {
                Self::COLORED
            } else {
                Self::PLAIN
            }
        }
        #[cfg(not(feature = "color"))]
        {
            let _ = stream;
            Self::PLAIN
        }
    }

    #[cfg(feature = "color")]
    fn wanted(stream: Stream) -> bool {
        use std::io::IsTerminal;
        let env = |k: &str| std::env::var_os(k);
        if env("NO_COLOR").is_some() {
            return false;
        }
        if let Some(force) = env("CLICOLOR_FORCE") {
            return force != "0";
        }
        if env("CLICOLOR").is_some_and(|v| v == "0") {
            return false;
        }
        if env("TERM").is_some_and(|t| t == "dumb") {
            return false;
        }
        match stream {
            Stream::Stdout => std::io::stdout().is_terminal(),
            Stream::Stderr => std::io::stderr().is_terminal(),
        }
    }
}
