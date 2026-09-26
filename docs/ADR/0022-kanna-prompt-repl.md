# ADR-0022: `kanna-prompt`, a REPL over the command definition

Status: Accepted (2026-09-26)

## Context

An interactive mode was wanted for CLIs built with kanna: the program
keeps running, reads a command per line, and executes it, the way
`sqlite3` or `redis-cli` do. Two questions: whether this belongs in the
`kanna` crate, and how much of a terminal layer to take on.

Everything a loop needs already exists in the definition: `try_parse_args`
takes an argument vector without the binary name, `Error` renders help,
usage, suggestions and value lists, and `check_examples` already had a
shell-like word splitter. What is missing is only the loop itself and
the conventions around it.

## Decision

* **A separate crate, `kanna-prompt`**, depending on `kanna` like the
  other generators (ADR-0015). Terminal input is a concern of its own,
  the `decl[help]` size budget has no headroom (ADR-0021), and the
  default dependency graph stays empty. The only change to `kanna` is
  making the word splitter public as `kanna::split_words`, the "public
  getter, never a private hook" rule of ADR-0015.
* **The loop is small and has no dependencies.** `Repl::new(&cmd)` with
  `prompt(..)` and `exit_words(..)`; `run(handler)` on the standard
  streams, `run_typed::<Args, _>` for `cli!`/derive structs, `run_with`
  on any reader and writers (tests, embedding). A line is an argument
  vector without the binary name. Empty and `#` lines are skipped;
  `exit`/`quit` alone end the loop unless the command defines a
  subcommand of that name; `help` alone becomes `--help`; end of input
  ends the loop, so a file of commands can be piped in; the prompt is
  shown only on a terminal. Errors go through `Error::print`, so colour
  and `KANNA_ERROR_FORMAT=json` behave as on the command line, and the
  loop continues. The handler returns `Flow::Continue` or `Flow::Exit`.
* **Line editing, history and completion are left to the caller.**
  `run_line(line, handler)` handles one line and returns errors instead
  of printing them, so a line-editing library (rustyline, reedline, ...)
  can drive the loop; the `complete` feature adds `complete_line(&cmd,
  partial)` built on `kanna_complete::dynamic::complete`, for such a
  library's completer. Taking a terminal dependency in `kanna-prompt`
  itself was rejected for now: it would pick one library for everyone
  and bring in crossterm-sized graphs.
* **Global options carry over by the application, not the loop.** The
  loop knows nothing about state; the `todo` example copies its outer
  `--file` into each parsed line when the line does not set one, and
  refuses `repl` inside the REPL. A `repl` subcommand is marked
  `no_tool` (ADR-0021) since it is for people.

Not done: prompting for missing arguments (a wizard mode). It would use
the same crate and the `MissingRequired` error's `arg`, and is left for
when someone needs it.

## Consequences

* A REPL costs one subcommand and a handler; help, errors and value
  checking are the command line's.
* `kanna` grows one public function; sizes are unchanged.
* The crate family is now six published crates plus `kanna-prompt`.
