# kanna compared with clap 4

Date: 2026-09-26, updated the same day after the parity work (ADR-0017,
ADR-0018). Measurements on a 24-core x86_64 Linux box with rustc 1.98.1,
clap 4.6.7 (clap_builder 4.6.7, clap_lex 1.1.1, clap_derive 4.6.7). Sizes
are the delta over an empty `main` built with the `size` profile
(`opt-level = "s"`, LTO, `codegen-units = 1`, `panic = "abort"`, stripped),
as in ADR-0007. Build times are clean builds of the sample binary including
all dependencies, one run each, so treat differences under about 0.2 s as
noise.

The sample CLI is the one in `benches/src/bin/`: one flag, two string
options, one `u32` with a default, one `OsString` option, one
`--color[=WHEN]` and one required positional.

## Numbers

### Binary size

| Variant | Before parity | After parity | Third-party crates |
|---------|--------------:|-------------:|-------------------:|
| kanna-core (imperative) | 13.5 KiB | 13.5 KiB | 0 |
| kanna builder, no features | 47.5 KiB | 57.1 KiB | 0 |
| kanna builder, `help` | 58.1 KiB | 74.1 KiB | 0 |
| kanna builder, `help + suggest` | 63.6 KiB | 80.1 KiB | 0 |
| kanna builder, `full` (help, suggest, color, env, json) | 72.3 KiB | 93.2 KiB | 0 |
| kanna `cli!`, `help` | 63.7 KiB | 81.4 KiB | 0 |
| kanna `cli!`, `full` | 78.1 KiB | 100.4 KiB | 0 |
| kanna derive, `help` | 63.7 KiB | 81 KiB | 4 (build-time only) |
| clap builder, minimal (`std, help, usage, error-context`) | 199.0 KiB | 199.0 KiB | 3 |
| clap builder, default features (adds `color`, `suggestions`) | 214.1 KiB | 214.1 KiB | 11 |
| clap derive, default features | 225.1 KiB | 225.1 KiB | 19 |

The parity features cost 13.7 KiB on the `help` build (ADR-0018 has the
breakdown) and the AI-oriented additions another 2.3 KiB, plus 5.6 KiB
for the `json` feature in `full` (ADR-0019). kanna with every feature
on is now 0.47 of clap with the fewest features on, and 0.44 of clap
with its defaults; against a comparable feature set (kanna `full`
versus clap default) the saving is about 120 KiB per binary. The `decl[help]` budget was raised from 60 to
75 KiB to admit this; the `full / clap < 0.5` gate is unchanged.

### Clean build time

| Variant | dev | release |
|---------|----:|--------:|
| kanna-core | 0.76 s | 1.06 s |
| kanna builder, `help` | 0.86 s | 1.31 s |
| kanna builder, `full` | 0.83 s | 1.40 s |
| kanna `cli!`, `full` | 0.87 s | 1.38 s |
| kanna derive, `help` (from README) | 2.3 s | 2.5 s |
| clap builder, minimal | 1.79 s | 2.73 s |
| clap builder, default features | 1.94 s | 3.12 s |
| clap derive, default features | 2.98 s | 3.16 s |

The builder and `cli!` fronts still build about twice as fast as clap.
The derive front does not: both pull in `syn`, which dominates the build.

### Size of the code to audit

| Crate | Lines of Rust | `unsafe` |
|-------|--------------:|:--------:|
| kanna-core | 857 | forbidden |
| kanna (builder, `cli!`, help, suggest, color, env) | 5,255 | forbidden |
| kanna-derive | 618 | forbidden |
| kanna-complete + kanna-doc + kanna-schema | 1,750 | forbidden |
| clap_lex | 810 | 5 blocks (`OsStr` splitting) |
| clap_builder | 28,992 | forbidden |
| clap_derive | 4,575 | forbidden |

kanna's parser (core + builder) is about one fifth the size of clap's,
and its `Cargo.toml` inherits `unsafe_code = "forbid"` for every crate.
Both projects have an MSRV of 1.85.

## Where kanna is better

* **Binary size.** Two fifths of clap's with all comparable features on.
  For a tool shipped as a single static binary, or for a project with
  many small binaries, that is the reason to pick it.
* **Build time.** Clean builds of the builder and `cli!` fronts take
  under a second against clap's 1.7 to 1.9 s, and incremental rebuilds of
  the application only recompile one small crate. Parsing speed is
  negligible in both.
* **Zero dependencies unless you opt into derive.** `cargo tree` for a
  `full`-featured kanna binary shows nothing but `std`. clap's default
  configuration brings 11 crates, the derive one 19.
* **A layered choice instead of one size.** Programs that just need a
  lexer take `kanna-core` at 13.5 KiB and grow into the builder without
  changing lexing rules. clap has no documented equivalent to the bottom
  layer.
* **Typed keys.** `m.get(&count)` returns `u32` because the `Arg<u32>`
  carries the type; `Option` versus plain versus `Vec` follows from
  `required()`, `default()` and `many()`. clap's `get_one::<u32>("count")`
  is checked at runtime and needs the id spelled in two places.
* **A macro DSL without proc macros.** `cli!` gives derive-style
  ergonomics, including `#[flatten]` and value enums, at builder-level
  build time and binary size.
* **Definition validation with a full report.** `Command::validate()`
  returns every problem at once, naming the command path and the
  argument, and runs automatically in debug builds. clap's
  `debug_assert` stops at the first.
* **Non-Unicode handling is uniform.** Values are `OsString` end to end.
* **GNU-style value taking.** `--retries -1` and `--name --weird` take
  the next argument as the value, as `getopt_long` does; clap needs
  `allow_hyphen_values`.
* **Definitions are plain values.** One `Command` serves the parser, the
  completion generator, the manpage generator and the JSON schema.
* **Auditability.** About 6,300 lines for lexer and builder together, no
  `unsafe` anywhere, 220 tests plus a 200,000-case pseudo-fuzz and a
  libFuzzer target.
* **Built for use with language models.** A one-page complete reference,
  compile errors that list the valid settings, verified examples, tool
  definitions and structured errors (table below). clap has none of
  these in the crate itself.

## Where clap is still better

* **Remaining feature gaps** (all judged rare, see ADR-0018): `Arg::last`
  (values only after `--`), `multicall` (busybox-style binaries),
  `value_hint` (kanna has dynamic completion callbacks instead),
  `default_value_if`, help templates, `next_line_help`, hidden possible
  values, `indices_of`, and terminal width *detection* (kanna reads
  `$COLUMNS` or takes `term_width`; clap queries the terminal).
* **Ecosystem and maturity.** clap has more than a decade of production
  use, tens of thousands of dependents, a tutorial, a cookbook, a FAQ,
  and companion crates. Bug reports and edge cases have been ironed out
  across every platform. kanna is at 0.1.0 with a single author, no
  published release, no external users, CI that has not yet run on
  Windows or macOS, and an API that may still change before 1.0.
* **Derive build time is not better.** Anyone choosing kanna for its
  derive front gets clap-like build times because `syn` dominates; only
  the size advantage remains.
* **The size margin is nearly spent.** `decl[help]` sits at 74.1 KiB
  under a 75 KiB budget and `full / clap` at 0.47 under 0.5; the next
  default-build feature forces a budget decision, whereas clap has no
  such constraint to manage.
* **Documentation volume.** API docs, README, migration guides and ADRs
  exist, but there is no tutorial and far fewer worked examples.

## Working with a language model

Since ADR-0019 this is an axis of its own. What each library offers to
a model that writes a CLI, and to an agent that calls one:

| Concern | clap 4 | kanna |
|---------|--------|--------|
| A complete, single-file list of every setting | none in the crate; the docs are a large rustdoc tree, a tutorial and a cookbook | `docs/ai/kanna-reference.md` plus a Claude Code skill |
| Wrong setting name in a derive/macro | derive: compile error listing valid attributes; builder: a runtime `debug_assert` for some mistakes | derive and `cli!`: compile error naming the setting and listing every valid one |
| Definition mistakes | `Command::debug_assert()`, first problem only | `Command::validate()`, every problem, automatic in debug builds |
| Examples in the definition | free text in `after_help` | `example = ".."`, rendered in help and docs and parsed by `check_examples()` |
| Exposing the CLI as an agent tool | not in clap itself | `kanna_schema::tool`: Claude / MCP tool definitions with JSON Schema inputs, and `to_argv` back to a validated command line |
| Machine-readable description of the CLI | not in clap itself | `kanna_schema::to_json` (format 2, with a JSON Schema) |
| Machine-readable errors | text only | `Error::to_json`, `Error::arg`, `ErrorKind::name`, `KANNA_ERROR_FORMAT=json` (feature `json`) |
| Suggestions for typos | `suggestions` feature | `suggest` feature (options, subcommands, values) |

The size cost of these additions is 2.3 KiB on the default build and
5.6 KiB more for `json`.

## Behavioural differences worth knowing

| Input | clap default | kanna |
|-------|--------------|--------|
| `--opt -1` (numeric option) | error, `-1` looks like a flag | takes `-1` as the value |
| `--opt --other` (string option) | error, missing value | takes `--other` as the value |
| `--opt a --opt b` (single value) | error, repeated | error, repeated (`last_wins()` opts out) |
| `--color always` for `--color[=WHEN]` | may consume `always` | `always` is a positional |
| `--lev` for `--level` | error unless `infer_long_args` | error unless `infer_long_args` |
| `prog help sub` | prints help | prints help |
| `-h` versus `--help` | short versus long help | short versus long help |
| invalid UTF-8 value | error unless `OsString` parser | accepted as `OsString`, error only on conversion |
| duplicate names in the definition | `debug_assert` panic, first problem | debug panic listing every problem |

## Recommendation

Pick kanna when binary size, dependency count or clean-build time is a
stated requirement and you are prepared to be an early adopter. The
feature set now covers what most command line tools need, including
flattened option structs, conditional requirements, value enums, help
headings and external subcommands. Pick clap when its ecosystem,
stability and support community matter more than roughly 130 KiB per
binary, or when one of the remaining gaps above is essential.

The two are close in API vocabulary (see `docs/migration/from-clap.md`),
so a program written against either can move to the other.
