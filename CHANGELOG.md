# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Until 1.0, minor versions may contain breaking changes; each comes with a
migration note here.

## [Unreleased]

### Added

* Example applications under `examples/` (`wc`, `sift`, `todo`, `hexdump`),
  one complete program per layer, and feature demonstrations under
  `hasami/examples/` (`values`, `nested`, `errors`, `env`, `wrapper`).
* `hasami-core`: dependency-free GNU/POSIX lexer (`Parser`, `Arg`,
  `ValueExt`, `RawArgs`, `ValuesIter`) with full non-Unicode support and a
  libFuzzer target.
* `hasami`: declarative `Command` / `Arg<T>` builder with typed values,
  `Group` constraints, `requires`, lazy subcommands, global options,
  `--help`/`--version` (feature `help`), "did you mean" tips (feature
  `suggest`), colour (feature `color`), environment fallback (feature
  `env`).
* `hasami::cli!`: `macro_rules!` DSL producing a typed struct and the same
  `Command`.
* `hasami-derive`: `#[derive(Args)]` and `#[derive(Commands)]` (feature
  `derive`) with `#[hasami(...)]` attributes; equivalence with `cli!` is
  tested.
* `hasami::Cli` and `hasami::Subcommands` traits implemented by both
  front ends.
* `hasami-complete`: static completion scripts for bash, zsh, fish,
  PowerShell and nushell, plus dynamic completion of values through
  `Arg::complete_with` and `complete_from_env`.
* `hasami-doc`: manpage (roff), Markdown and HTML generation.
* `hasami-schema`: JSON description of everything a command accepts, with a
  JSON Schema for the format.
* Size and build-time gates (`benches/`), CI workflow, `cargo deny`
  configuration, trycmd snapshot tests, examples for every layer.
