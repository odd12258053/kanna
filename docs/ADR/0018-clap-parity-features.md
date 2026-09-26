# ADR-0018: Closing the feature gaps against clap

Status: Accepted (2026-09-26). Amends ADR-0010 and ADR-0012.

## Context

`docs/reports/comparison-with-clap.md` listed clap capabilities with no
kanna equivalent. The user asked for them to be closed. Each item below
was weighed against the size budget; the sum did not fit in the old
60 KiB gate, which is addressed at the end.

## Decision

Everything is available in the builder and, where it applies to a field
or a struct, in `cli!` and `#[derive(Args)]` under the same name.

| Gap | kanna now |
|-----|------------|
| `#[command(flatten)]` | `#[flatten] field: Struct` inlines another definition's arguments, groups and constraints; `from_matches` reads them back from the same `Matches`. |
| Several values per occurrence (`num_args(1..)`) | `Arg<Vec<T>>::greedy()`: each occurrence takes every following non-option argument, via the core's `Parser::values`. An attached value (`--exec=cmd`) limits that occurrence to one value. |
| Value delimiters | `Arg<Vec<T>>::delimiter(',')`: split before parsing; non-Unicode values are not split. |
| `required_if_eq`, `required_unless_present`, `requires_if`, `requires`, `conflicts_with` on an argument | `Arg::required_if_eq`, `required_unless`, `requires_if`, `requires`, `conflicts_with`, stored as one `Vec<Relation>` per argument (a single enum keeps `ArgDef`'s clone and drop glue small). `required_unless` overrides `required`. Raw values are compared, and only explicitly given arguments count. |
| `ValueEnum` | The `ValueEnum` trait (`VALUES`, `name()`), the `value_enum!` macro for `macro_rules!` users and `#[derive(ValueEnum)]` with `derive`. Both also implement `FromStr` and `Display`. `Arg::value_enum::<T>()` and the `value_enum` field setting take the possible values from the type, so help and parsing cannot disagree. |
| `-h` versus `--help`, `long_help`, `long_about` | `-h` shows `about` and `help`; `--help` shows `long_about` and `long_help` where set (falling back to the short forms). `Command::render_short_help` renders the `-h` form. |
| `help_heading` | `Arg::help_heading("Title")`. Custom sections follow `Options` in first-seen order. |
| Visible aliases | `Arg::visible_alias`, `Subcommand::visible_alias`, rendered as `[aliases: a, b]`. |
| `before_help`, `long_version` | `Command::before_help` (printed first) and `Command::long_version` (`--version` prints it, `-V` keeps the short version). |
| Terminal-width wrapping | Help paragraphs and the description column wrap at `Command::term_width`, else `$COLUMNS`, else 100 columns; `0` disables. Explicit line breaks and leading spaces are kept, so hand-formatted examples survive. |
| `help` subcommand | `app help [SUB...]` shows the help of the deepest resolvable level when the command has subcommands and help is enabled. A user-defined `help` subcommand wins. |
| `arg_required_else_help` | `Command::arg_required_else_help()`: with no arguments at all, help is printed to stderr and the status is 2 (`ErrorKind::HelpOnMissingArgs`, not a display error). |
| `infer_long_args`, `infer_subcommands` | `Command::infer_long_args()` and `infer_subcommands()`: an unambiguous prefix is accepted; exact matches win; ambiguity is the usual unknown-name error. Off by default, as in clap. |
| External subcommands | `Command::allow_external_subcommands()`: an unknown first positional and everything after it come back through `Matches::external_subcommand()`, untouched, and satisfy `subcommand_required`. |
| `trailing_var_arg` / `raw` | `Arg<Vec<T>>::trailing()` on a repeated positional: after its first value every remaining argument is a value, options included. |
| Custom colour styles | `Command::styles(Styles)` with `Styles::with_header(..)` etc.; used by help and error output when colour is on. |
| `ArgMatches::ids` | `Matches::ids()`. |

Not done, deliberately: `Arg::last` (only after `--`; `--` already works
and `trailing` covers the rest), `multicall`, `value_hint` (dynamic
completion callbacks exist), `default_value_if`, help templates,
`next_line_help`, hidden possible values, `indices_of`. Each is either
rare or expressible in application code.

## Amendments to earlier ADRs

* ADR-0010: help text **is** wrapped now (default 100 columns); the
  `-h`/`--help` forms differ where long texts exist; custom headings and
  `[aliases: ..]` were added to the layout.
* ADR-0012: the `decl[help]` size budget rises from 60 KiB to 75 KiB.
  The features above cost 13.7 KiB on the sample CLI (58.1 → 71.8 KiB),
  most of it in the help renderer (headings, wrapping, long/short forms)
  and the parse loop (relations, greedy values, inference, external
  subcommands). Validation is free in release builds. The
  `decl[full] / clap < 0.5` gate is unchanged and measures 0.44.
  SPEC.md §5 states 60 KiB as a target; this ADR records the deliberate
  departure and the reason.

## Consequences

* Users of the default features pay about 14 KiB more per binary than
  before; `kanna-core` and the builder without `help` are unaffected in
  kind (57.1 KiB without features, up from 47.5).
* The `cli!`/derive vocabulary grew by 16 field settings and 9 command
  settings; the `cli!` documentation lists them all.
* The JSON schema format is now version 2 with the new fields.
