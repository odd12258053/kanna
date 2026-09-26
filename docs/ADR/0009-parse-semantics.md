# ADR-0009: Parse semantics of the declarative layer

Status: Accepted (2026-09-26). Amended by ADR-0017 (repeats) and ADR-0018 (new settings).

## Context

The lexer (ADR-0003) never decides whether an option takes a value; the
declarative layer does. Several behaviours are conventions rather than
standards and must be fixed.

## Decision

| Situation | Behaviour |
|-----------|-----------|
| Option repeated, single value (`-n1 -n2`) | Error `Repeated` (ADR-0017). `Arg::last_wins()` or `Command::args_override_self()` restores last-wins; `Matches::occurrences` reports 2 either way. |
| Flag repeated (`--shout --shout`) | Error `Repeated` unless `.count()`, `last_wins()` or `args_override_self()`. |
| Flag given a value (`--shout=yes`) | Error `UnexpectedValue`, wording "which takes no value". Comes for free from the core's pending-value check. |
| Optional value (`.default_missing`) | Only an attached value counts (`--color=never`, `-cnever`); `--color never` treats `never` as the next positional. Same rule as `getopt_long` and the only unambiguous one. |
| Value looking like an option (`-n -5`) | Taken as the value, per ADR-0003. |
| First positional that names a subcommand | Subcommand wins; otherwise it fills the next positional slot. A command with subcommands and no free positional slot reports `UnknownSubcommand`. |
| Options after a subcommand | Resolved in the subcommand first, then in enclosing commands but only for `global()` args. |
| `-h`/`--help`, `-V`/`--version` | Synthetic, feature `help`; a user-defined arg with the same name wins. Help is rendered for the innermost command on the path (`app add --help`). Reported as `Error` with `ErrorKind::DisplayHelp`/`DisplayVersion`, exit status 0, so `try_parse` callers stay in control of I/O. |
| Missing required arguments | Reported together, one per line, after all arguments are consumed, so the user fixes them in one round. Checked per command from the root down. |
| Environment fallback (feature `env`) | Consulted only when the argument is absent from the command line, before defaults; the value goes through the same validation and counts as "present". |
| Error usage line | The usage of the command level where the error happened (`repo add [OPTIONS] <NAME>`), followed by `For more information, try '--help'.` when help is enabled. |
| Error wording | `error: <what> [<where>]` on one line, an optional `tip:` line with how to fix it, then usage. Suggestions (feature `suggest`) go in the tip. |
| Non-Unicode values | Passed to `value_os`/`positional_os` parsers untouched; `FromStr`-based parsers report `ErrorKind::NonUnicode` naming the option. |

## Consequences

* The declarative layer uses only the public API of the core; nothing was
  added to the core for it.
* All of the above is pinned by `kanna/tests/builder.rs`.
