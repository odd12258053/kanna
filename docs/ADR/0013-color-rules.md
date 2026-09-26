# ADR-0013: Colour output rules

Status: Accepted (2026-09-26)

## Context

SPEC.md asks for an optional `color` feature honouring `NO_COLOR`,
`CLICOLOR` and TTY detection. Colour must never change the text, must be
absent from `Display` output (which tests and log files rely on), and must
cost nothing when the feature is off.

## Decision

* `Styles` holds seven escape-sequence strings; `Styles::PLAIN` is all
  empty. Without the `color` feature the struct is empty and every accessor
  is a constant `""`, so the rendering code compiles to the plain version.
* The palette is decided per stream by `Styles::for_stream`:
  1. `NO_COLOR` set → off (https://no-color.org);
  2. `CLICOLOR_FORCE` set → on unless it is `0`;
  3. `CLICOLOR=0` → off;
  4. `TERM=dumb` → off;
  5. otherwise on iff the stream is a terminal.
* `Error::print` applies the stream's palette; `Display` is always plain;
  `Error::render(&Styles)` lets callers choose. To re-render help and usage
  with colour, the error keeps a clone of its `Command` (only with the
  feature on).
* Palette: bold+underline section headers, bold literals (`--name`,
  subcommand names, `--help` in the footer), cyan placeholders (`<NAME>`),
  bold red `error:`, green `tip:`.
* Stripping all escapes from a coloured rendering yields exactly the plain
  rendering; a test checks this.

## Consequences

* Snapshot tests keep using `Display` and never see escapes.
* Windows 10+ consoles accept these sequences; older consoles are not
  special-cased (users can set `NO_COLOR`).
