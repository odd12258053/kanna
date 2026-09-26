# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Until 1.0, minor versions may contain breaking changes; each comes with a
migration note here.

## [Unreleased]

### Added

* `kanna-prompt` (ADR-0022): a read-eval-print loop over a `Command`.
  `Repl::new(&cmd).prompt("app> ").run(handler)` reads lines from
  standard input, splits them like a shell, parses each with the same
  definition and calls the handler with the `Matches` (`run_typed` for
  a `cli!`/derive struct). `help`, `exit`/`quit`, `#` comments, end of
  input, errors printed and the loop continued. `run_line` for
  line-editing front ends; feature `complete` adds `complete_line`.
* `kanna::split_words`: the shell-like splitter used by
  `check_examples` and the REPL, now public.
* `examples/todo` gains a `repl` subcommand.

## [0.2.0] - 2026-09-26

### Added

* Descriptions for possible values (ADR-0021): a `///` comment on a
  `value_enum!` or `#[derive(ValueEnum)]` variant is its
  `ValueEnum::help()`, `Arg::possible_with_help([("c", "Celsius")])`
  does the same for plain values, `ArgDef::possible_value_help()` reads
  them back. `--help` (not `-h`) shows `[possible values: c = Celsius,
  f]`; tool definitions append the same legend to the property
  description.
* `no_tool` on subcommands (`Subcommand::no_tool()`, `#[no_tool]` in
  `cli!`, `#[kanna(no_tool)]` in derive): listed in help and completion
  but not offered to AI agents as a tool.
* `ValueType::Unsigned` for `u8` .. `u128` and `usize`; tool schemas give
  such arguments `minimum: 0`.
* A `MissingRequired` error names the argument (`Error::arg`) when
  exactly one is missing.
* `examples/units`: a unit converter an AI agent can drive. Shows
  `kanna_schema::tool` end to end: `tools` prints Claude / MCP tool
  definitions, `call` turns a tool's JSON input into a command line with
  `to_argv` and runs it, `schema` prints the JSON description.

### Changed

* **Breaking:** `kanna_schema::tool::to_argv` returns
  `Result<_, kanna::Error>` instead of `Result<_, String>`, so a driver
  can branch on `kind()` and `arg()` and print the error as text or JSON
  like any other. Parser errors pass through untouched; a bad key is
  `UnexpectedArgument` and a wrong value shape `InvalidValue`, both with
  the key as `arg`; malformed JSON and an unknown tool are `Custom`.
* **Breaking:** the `kanna-schema` document is format 3: `value_type`
  may be `"unsigned"` (signed types stay `"integer"`), arguments carry
  `possible_value_help` (an object of value → text) and subcommands
  `no_tool`.

## [0.1.0] - 2026-09-26

First release on crates.io.

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

[Unreleased]: https://github.com/odd12258053/kanna/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/odd12258053/kanna/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/odd12258053/kanna/releases/tag/v0.1.0
