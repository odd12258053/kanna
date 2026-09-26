# ADR-0017: Repeated arguments are errors; definitions are validated

Status: Accepted (2026-09-26). Amends ADR-0009.

## Context

The comparison with clap (`docs/reports/comparison-with-clap.md`) named
two defaults where hasami caught fewer user and programmer mistakes than
clap:

* A single-value option or a flag given twice (`-n1 -n2`, `--shout
  --shout`) silently took the last value (ADR-0009, "GNU style").
* A malformed definition (duplicate ids or names, a positional after a
  repeated positional, a group naming an unknown argument, a default
  outside the possible values) was only partly caught, by two
  `debug_assert!`s in `Command::arg`, and otherwise reached production.

## Decision

1. **Repeating is an error by default.** Giving a single-value argument
   or a plain flag more than once yields `ErrorKind::Repeated` with the
   message `the argument '--number <NUMBER>' cannot be used multiple
   times`. Counters (`.count()`) and repeated arguments (`.many()`) are
   unaffected. The old behaviour is an opt-in: `Arg::last_wins()` per
   argument, or `Command::args_override_self()` for a whole command; the
   `cli!`/derive settings are `last_wins` and `args_override_self`.
   `Matches::occurrences` keeps counting in either mode.

   Rationale: a repeated option is nearly always a mistake in a script or
   a shell alias gone wrong, and clap users expect the error. The GNU
   convention is still one setting away.

2. **`Command::validate()` checks the whole definition** and returns
   every problem as `command path: description`. It checks: empty or
   duplicate ids; duplicate long names (including aliases) and short
   names; invalid names (`-` or whitespace in a short, `=`, space or a
   leading `-` in a long); positionals with a long or short name, marked
   global, or with an optional value; a positional after a repeated one;
   a required positional after an optional one; `trailing` without
   `many`; `greedy`/`trailing` on the wrong kind of argument; `count`
   combined with a value; a default outside the possible values; a
   delimiter on a single-value argument; relations, `requires` and groups
   naming unknown ids; exclusive groups with fewer than two members;
   duplicate subcommand names or aliases; `subcommand_required` without
   subcommands; a required positional next to subcommands; and the same
   for every subcommand, building lazy ones.

   The synthetic `-h/--help` and `-V/--version` are **not** reserved
   names: a user-defined argument with one of them replaces the flag, as
   ADR-0009 already said.

3. **Parsing runs `validate()` in debug builds** (`cfg!(debug_assertions)`)
   and panics with the full list. Release builds skip it, so the check
   costs nothing in the shipped binary (the size gate confirmed that the
   code is eliminated). The two `debug_assert!`s in `Command::arg` were
   removed in favour of the central check.

## Consequences

* Existing programs that relied on last-wins see an error until they add
  `last_wins()`; the migration guide says so.
* A definition mistake shows up in the first test or debug run, with a
  message naming the command and the argument.
* `hasami/tests/extended.rs` pins the messages; `hasami/tests/builder.rs`
  pins the repeat behaviour.
