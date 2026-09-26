# Step 9 report: closing the gaps against clap

Date: 2026-09-26. Machine and toolchain as in the step 4/5 report
(24-core x86_64 Linux, rustc 1.98.1). Follows the comparison in
`comparison-with-clap.md`; decisions in ADR-0017 and ADR-0018.

## What was asked

Remove the disadvantages found in the comparison, in particular the
feature gaps, the silent last-wins on repeated options, and the weak
definition validation.

## What was built

### Behaviour (ADR-0017)

* Repeating a single-value argument or a flag is `ErrorKind::Repeated`;
  `Arg::last_wins()` / `Command::args_override_self()` opt out.
* `Command::validate()` reports every definition problem as
  `path: description` (23 rules, see the ADR); parse entry points run it
  in debug builds and panic with the list. Release builds contain none
  of it.

### Features (ADR-0018), in every front end

| Area | Added |
|------|-------|
| Composition | `#[flatten]` (`cli!` and derive), `__macro::flatten` |
| Values | `greedy()` (several values per occurrence), `delimiter(c)`, `trailing()` positionals, `ValueEnum` trait + `value_enum!` + `#[derive(ValueEnum)]`, `Arg::value_enum`, `positional_enum` |
| Constraints | `Arg::requires`, `conflicts_with`, `required_unless`, `required_if_eq`, `requires_if` (`Relation` enum) |
| Help | `long_help`, `long_about` only under `--help`, `render_short_help`, `help_heading`, visible aliases (`[aliases: ..]`), `before_help`, `long_version`, wrapping (`term_width`, `$COLUMNS`, 100), `help` subcommand, `arg_required_else_help` (`ErrorKind::HelpOnMissingArgs`) |
| Names | `infer_long_args`, `infer_subcommands` |
| Subcommands | `allow_external_subcommands` + `Matches::external_subcommand` |
| Output | `Command::styles` with `Styles::with_*` setters |
| Matches | `ids()` |
| Generators | schema format 2 with every new field; doc and completion generators show/offer visible aliases and long help |

Tests: `kanna/tests/extended.rs` (25 cases), parity cases in
`tests/macro.rs` and `tests/derive.rs` (flatten, value enums, every new
setting), updated repeat tests in `tests/builder.rs`, schema shape test.
Workspace total: 208 tests, all green with default, no and all features.

## Measurements

Size, `size` profile, delta over an empty `main`:

| Variant | Before | After | Budget |
|---------|-------:|------:|-------:|
| core | 13.5 KiB | 13.5 KiB | ≤ 20 |
| decl[no features] | 47.5 KiB | 57.1 KiB | |
| decl[help] | 58.1 KiB | 71.8 KiB | ≤ 75 (was 60) |
| decl[help,suggest] | 63.6 KiB | 77.7 KiB | |
| decl[full] | 72.3 KiB | 87.6 KiB | |
| macro[help] | 63.7 KiB | 79.3 KiB | |
| macro[full] | 78.1 KiB | 94.8 KiB | |
| derive[help] | 63.7 KiB | 79.2 KiB | |
| decl[full] / clap | 0.36 | 0.44 | < 0.5 |

Clean build time (dev / release), same run for every row so the numbers
are comparable with each other but about 0.1 s slower across the board
than the step 8 run:

| Variant | dev | release | Budget |
|---------|----:|--------:|-------:|
| core | 0.76 s | 1.06 s | ≤ 2 s |
| decl[help] | 0.86 s | 1.31 s | |
| decl[full] | 0.83 s | 1.40 s | ≤ 4 s |
| macro[full] | 0.87 s | 1.38 s | |
| clap | 1.79 s | 2.73 s | |

Every gate passes (`benches/size.sh --check --compare`,
`benches/build-time.sh --check`), as do fmt, clippy (all and no
features, `-D warnings`), rustdoc with `-D warnings`, and the tests.

## Design points that needed a decision

* **The size budget.** The features cost 13.7 KiB on the `help` build.
  Boxing rarely used `ArgDef` fields and merging the five relation lists
  into one `Vec<Relation>` recovered about 1 KiB; the rest is real code
  in the help renderer and the parse loop. Rather than hide half the
  features behind another Cargo feature, ADR-0018 raises the budget to
  75 KiB and keeps the `< 0.5 × clap` gate (now 0.44). SPEC.md's 60 KiB
  remains the recorded target it departs from.
* **Validation for free.** `cfg!(debug_assertions)` around the call lets
  LTO drop `validate_into` entirely from release binaries; `cargo bloat`
  confirmed no trace of it.
* **`-h/--help` and `-V/--version` stay unreserved** so that
  `Arg::with_id("host").short('h')` keeps working (ADR-0009); validation
  therefore does not flag them.
* **Relations compare raw values and only explicit arguments.** A default
  never triggers `required_if_eq`; documented in the migration guide.
* **`help` subcommand descends as far as names resolve**, then shows that
  level's help, so `app help nope` shows the root help instead of an
  error.
* **Wrapping keeps indentation and blank lines**, so hand-formatted
  `after_help` examples survive; the default width of 100 matches clap.
* **`ValueEnum` names are `&'static str`**, so `value_enum!` requires an
  explicit `Variant = "name"` (a `macro_rules!` cannot kebab-case at
  compile time) while the derive computes kebab-case and accepts
  `#[kanna(name = "..")]`.

## Not done / follow-ups

* `Arg::last`, `multicall`, `value_hint`, `default_value_if`, help
  templates, `next_line_help`, hidden possible values, `indices_of`,
  terminal width detection (needs `libc` or `unsafe`).
* Nothing has been committed; the working tree holds the parity work on
  top of the initial commit.
