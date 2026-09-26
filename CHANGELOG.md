# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Until 1.0, minor versions may contain breaking changes; each comes with a
migration note here.

## [Unreleased]

### Changed

* The project is named **kanna** (was hasami, which was already taken on
  crates.io): crates `kanna`, `kanna-core`, `kanna-derive`,
  `kanna-complete`, `kanna-doc`, `kanna-schema`; attribute
  `#[kanna(...)]`; environment variables `KANNA_ERROR_FORMAT` and
  `_KANNA_COMPLETE*` (ADR-0020).

### Added

* For AI-assisted development (ADR-0019): `docs/ai/kanna-reference.md`
  (every name and setting on one page) and a Claude Code skill;
  `cli!` now reports an unknown setting with the list of valid ones;
  `Command::example` / `example = ".."` with an `Examples:` help section
  and `Command::check_examples()`; `ValueType` recorded per argument;
  `kanna_schema::tool` (Claude / MCP tool definitions from a `Command`,
  and `to_argv` back to a command line); feature `json` with
  `Error::to_json`, `Error::arg`, `ErrorKind::name` and
  `KANNA_ERROR_FORMAT=json`.
* Feature parity with clap where it was missing (ADR-0018): `#[flatten]`,
  `greedy()` values per occurrence, `delimiter()`, per-argument
  `requires` / `conflicts_with` / `required_unless` / `required_if_eq` /
  `requires_if` (`Relation`), the `ValueEnum` trait with `value_enum!`
  and `#[derive(ValueEnum)]`, `long_help` / `long_about` shown only by
  `--help`, `help_heading`, visible aliases, `before_help`,
  `long_version`, help wrapping (`term_width`, `$COLUMNS`, 100), the
  `help` subcommand, `arg_required_else_help`, `infer_long_args`,
  `infer_subcommands`, `allow_external_subcommands` with
  `Matches::external_subcommand`, `trailing()` positionals, custom
  `Styles` on a command, `Matches::ids`, `Command::render_short_help`.
* `Command::validate()`: every rule about a well-formed definition,
  reported together; run automatically in debug builds (ADR-0017).
* `ErrorKind::Repeated` and `ErrorKind::HelpOnMissingArgs`.
* JSON schema format 2 with the new fields.

### Changed

* A single-value argument or a flag given more than once is an error;
  `Arg::last_wins()` / `Command::args_override_self()` restore the old
  behaviour (ADR-0017).
* Help text is wrapped to the terminal width by default; `-h` and
  `--help` differ when long texts are set (ADR-0018).
* The `decl[help]` size budget is 75 KiB (was 60); the sample CLI
  measures 71.8 KiB (ADR-0018).

* Example applications under `examples/` (`wc`, `sift`, `todo`, `hexdump`),
  one complete program per layer, and feature demonstrations under
  `kanna/examples/` (`values`, `nested`, `errors`, `env`, `wrapper`).
* `kanna-core`: dependency-free GNU/POSIX lexer (`Parser`, `Arg`,
  `ValueExt`, `RawArgs`, `ValuesIter`) with full non-Unicode support and a
  libFuzzer target.
* `kanna`: declarative `Command` / `Arg<T>` builder with typed values,
  `Group` constraints, `requires`, lazy subcommands, global options,
  `--help`/`--version` (feature `help`), "did you mean" tips (feature
  `suggest`), colour (feature `color`), environment fallback (feature
  `env`).
* `kanna::cli!`: `macro_rules!` DSL producing a typed struct and the same
  `Command`.
* `kanna-derive`: `#[derive(Args)]` and `#[derive(Commands)]` (feature
  `derive`) with `#[kanna(...)]` attributes; equivalence with `cli!` is
  tested.
* `kanna::Cli` and `kanna::Subcommands` traits implemented by both
  front ends.
* `kanna-complete`: static completion scripts for bash, zsh, fish,
  PowerShell and nushell, plus dynamic completion of values through
  `Arg::complete_with` and `complete_from_env`.
* `kanna-doc`: manpage (roff), Markdown and HTML generation.
* `kanna-schema`: JSON description of everything a command accepts, with a
  JSON Schema for the format.
* Size and build-time gates (`benches/`), CI workflow, `cargo deny`
  configuration, trycmd snapshot tests, examples for every layer.
