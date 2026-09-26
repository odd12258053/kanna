# ADR-0010: Help output format

Status: Accepted (2026-09-26)

## Context

Help text is user-facing, snapshot-tested, and consumed by people who use
many CLIs. Inventing a new layout would cost users more than it gains.

## Decision

Follow the clap 4 layout, which is the most widely recognised in the Rust
ecosystem:

```
<about or long_about>

Usage: <path> [OPTIONS] --required <R> <POSITIONAL> [OPTIONAL]... [COMMAND]

Commands:
  name  summary

Arguments:
  <POSITIONAL>  help

Options:
  -s, --long <VALUE>  help [default: x] [possible values: a, b] [env: VAR]
      --long-only     help
  -h, --help          Print help
  -V, --version       Print version

<after_help>
```

* Column width is the widest left column in the section (capped at 36
  characters); a longer entry puts its help on the next line.
* Sections that would be empty are omitted; no trailing spaces are emitted.
* `[OPTIONS]` appears when any optional named argument exists (including
  the synthetic help/version flags). Required options are spelled out in
  the usage line.
* Optional values render as `--color[=<WHEN>]`; repeated values as
  `--include <INCLUDE>...`; repeated positionals as `[FILE]...`.
* Colour (feature `color`) is applied to the same structure, never changing
  the text.

## Consequences

* Snapshot tests can be compared against clap output by eye when migrating.
* Line wrapping of long help strings to the terminal width is not done;
  authors control line breaks with `\n` in the help text.
