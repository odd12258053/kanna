//! `rpn`: a reverse Polish calculator that runs as a session. The program
//! has two definitions: `Args` for the command line that starts it
//! (`--precision`, `--quiet`), and `Line` for what can be typed at the
//! prompt (`push`, `add`, `show`, ...). `kanna-prompt` runs the second
//! one: each line is split, parsed and checked like a command line, so
//! `push x` or `ad` get the parser's own messages and suggestions, and
//! `help` lists the verbs.
//!
//! ```text
//! cargo run -p kanna-example-rpn
//! cargo run -p kanna-example-rpn -- --precision 4
//! printf 'push 3 4\nadd\npush 2\nmul\nshow\n' | cargo run -q -p kanna-example-rpn
//! printf 'push 1\npop --all\npopp\n' | cargo run -q -p kanna-example-rpn
//! ```
//!
//! The stack lives in the handler closure; `exit`, `quit`, `q` or end of
//! input end the session. Nothing here is an agent tool, so there is no
//! `no_tool` to mark: the session definition is never given to
//! `kanna-schema`.
#![forbid(unsafe_code)]

use std::io;

use kanna::{Cli, Error};
use kanna_prompt::{Flow, Repl};

kanna::cli! {
    /// A reverse Polish calculator session
    #[name = "rpn", version = env!("CARGO_PKG_VERSION")]
    #[after_help = "Type 'help' at the prompt for the verbs, 'exit' to leave."]
    #[example = "rpn --precision 4"]
    #[derive(Debug)]
    struct Args {
        /// Decimal places shown
        #[short, default = 2]
        precision: usize,
        /// Do not print the stack after each verb
        #[short]
        quiet: bool,
    }
}

kanna::cli! {
    /// One line of the session
    #[name = "rpn"]
    #[derive(Debug)]
    struct Line {
        #[subcommand]
        verb: Verb,
    }

    #[derive(Debug)]
    enum Verb {
        /// Push one or more numbers
        Push(Push),
        /// Remove the top value
        Pop(Pop),
        /// Add the top two values
        Add,
        /// Subtract the top value from the one below
        Sub,
        /// Multiply the top two values
        Mul,
        /// Divide the value below by the top value
        Div,
        /// Duplicate the top value
        Dup,
        /// Swap the top two values
        Swap,
        /// Empty the stack
        Clear,
        /// Print the stack, bottom to top
        #[alias = "p"]
        Show,
    }

    #[derive(Debug)]
    struct Push {
        /// The values, bottom first
        #[positional, required, value_name = "N"]
        values: Vec<f64>,
    }

    #[derive(Debug)]
    struct Pop {
        /// Remove everything
        #[short]
        all: bool,
    }
}

/// The session state and the settings that shape its output.
struct Session {
    stack: Vec<f64>,
    precision: usize,
    quiet: bool,
}

impl Session {
    /// Take the top two values (`below`, `top`) for a binary verb.
    fn two(&mut self) -> Result<(f64, f64), Error> {
        if self.stack.len() < 2 {
            return Err(Error::custom("the verb needs two values on the stack")
                .with_tip("push some numbers first"));
        }
        let top = self.stack.pop().unwrap_or(0.0);
        let below = self.stack.pop().unwrap_or(0.0);
        Ok((below, top))
    }

    fn top(&self) -> Result<f64, Error> {
        self.stack
            .last()
            .copied()
            .ok_or_else(|| Error::custom("the stack is empty"))
    }

    /// Apply one parsed line. The stack text to print, if any.
    fn apply(&mut self, line: Line) -> Result<Option<String>, Error> {
        match line.verb {
            Verb::Push(p) => self.stack.extend(p.values),
            Verb::Pop(p) => {
                if p.all {
                    self.stack.clear();
                } else {
                    self.top()?;
                    self.stack.pop();
                }
            }
            Verb::Add => {
                let (a, b) = self.two()?;
                self.stack.push(a + b);
            }
            Verb::Sub => {
                let (a, b) = self.two()?;
                self.stack.push(a - b);
            }
            Verb::Mul => {
                let (a, b) = self.two()?;
                self.stack.push(a * b);
            }
            Verb::Div => {
                let (a, b) = self.two()?;
                if b == 0.0 {
                    self.stack.push(a);
                    self.stack.push(b);
                    return Err(Error::custom("division by zero"));
                }
                self.stack.push(a / b);
            }
            Verb::Dup => {
                let t = self.top()?;
                self.stack.push(t);
            }
            Verb::Swap => {
                let (a, b) = self.two()?;
                self.stack.push(b);
                self.stack.push(a);
            }
            Verb::Clear => self.stack.clear(),
            Verb::Show => return Ok(Some(self.render())),
        }
        Ok(if self.quiet {
            None
        } else {
            Some(self.render())
        })
    }

    fn render(&self) -> String {
        if self.stack.is_empty() {
            return "(empty)".to_owned();
        }
        let p = self.precision;
        self.stack
            .iter()
            .map(|v| format!("{v:.p$}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Build the loop; shared by `main` and the tests.
fn repl(cmd: &kanna::Command) -> Repl<'_> {
    Repl::new(cmd)
        .prompt("rpn> ")
        .exit_words(["exit", "quit", "q"])
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mut session = Session {
        stack: Vec::new(),
        precision: args.precision,
        quiet: args.quiet,
    };
    let cmd = Line::command();
    repl(&cmd).run_typed::<Line, _>(|line| {
        if let Some(text) = session.apply(line)? {
            println!("{text}");
        }
        Ok(Flow::Continue)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_are_sound() {
        for cmd in [Args::command(), Line::command()] {
            cmd.validate().unwrap();
            cmd.check_examples().unwrap();
        }
    }

    /// Feed `input` to a fresh session; the stack lines printed, the
    /// errors, and the final stack.
    fn session(input: &str, quiet: bool) -> (Vec<String>, String, Vec<f64>) {
        let mut s = Session {
            stack: Vec::new(),
            precision: 1,
            quiet,
        };
        let mut printed = Vec::new();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let cmd = Line::command();
        repl(&cmd)
            .run_with(input.as_bytes(), &mut out, &mut err, |m| {
                if let Some(text) = s.apply(Line::from_matches(m)?)? {
                    printed.push(text);
                }
                Ok(Flow::Continue)
            })
            .unwrap();
        (printed, String::from_utf8(err).unwrap(), s.stack)
    }

    #[test]
    fn arithmetic() {
        let (printed, err, stack) = session("push 3 4\nadd\npush 2\nmul\ndup\n", false);
        assert_eq!(printed, ["3.0 4.0", "7.0", "7.0 2.0", "14.0", "14.0 14.0"]);
        assert_eq!(err, "");
        assert_eq!(stack, [14.0, 14.0]);
        let (printed, _, _) = session("push 1 2\nsub\nshow\n", true);
        assert_eq!(printed, ["-1.0"]);
    }

    #[test]
    fn errors_come_from_the_parser_and_the_session() {
        let (printed, err, stack) = session("push x\nadd\npush 1 0\ndiv\npopp\nq\npush 9\n", false);
        assert_eq!(printed, ["1.0 0.0"]);
        assert!(err.contains("invalid value 'x' for '<N>'"), "{err}");
        assert!(err.contains("the verb needs two values"), "{err}");
        assert!(err.contains("division by zero"), "{err}");
        assert!(err.contains("unrecognized subcommand 'popp'"), "{err}");
        assert!(err.contains("a similar subcommand exists: 'pop'"), "{err}");
        // `q` ended the session before `push 9`; `div` left its operands.
        assert_eq!(stack, [1.0, 0.0]);
    }

    #[test]
    fn help_lists_the_verbs() {
        let mut out = Vec::new();
        let cmd = Line::command();
        repl(&cmd)
            .run_with("help\n".as_bytes(), &mut out, Vec::new(), |_| {
                Ok(Flow::Continue)
            })
            .unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("  push   Push one or more numbers"), "{out}");
        assert!(
            out.contains("  show   Print the stack, bottom to top"),
            "{out}"
        );
    }
}
