# kanna compared with clap 4

Date: 2026-09-26, for kanna 0.2.1 (the first version of this report
covered the unreleased 0.1.0 after the parity work of ADR-0017 and
ADR-0018; this one adds ADR-0019 to ADR-0022). Measurements on a 24-core
x86_64 Linux box with rustc 1.98.1, clap 4.6.7 (clap_builder 4.6.7,
clap_lex 1.1.1, clap_derive 4.6.7). Sizes are the delta over an empty
`main` built with the `size` profile (`opt-level = "s"`, LTO,
`codegen-units = 1`, `panic = "abort"`, stripped), as in ADR-0007. Build
times are clean builds of the sample binary including all dependencies,
one run each, so treat differences under about 0.2 s as noise.

The sample CLI is the one in `benches/src/bin/`: one flag, two string
options, one `u32` with a default, one `OsString` option, one
`--color[=WHEN]` and one required positional.

## Numbers

### Binary size

| Variant | 0.1.0 (pre-release) | 0.2.1 | Third-party crates |
|---------|--------------------:|------:|-------------------:|
| kanna-core (imperative) | 13.5 KiB | 13.5 KiB | 0 |
| kanna builder, no features | 57.1 KiB | 59.3 KiB | 0 |
| kanna builder, `help` | 74.1 KiB | 75.0 KiB | 0 |
| kanna builder, `help + suggest` | 80.1 KiB | 80.9 KiB | 0 |
| kanna builder, `full` (help, suggest, color, env, json) | 93.2 KiB | 93.9 KiB | 0 |
| kanna `cli!`, `help` | 81.4 KiB | 82.6 KiB | 0 |
| kanna `cli!`, `full` | 100.4 KiB | 101.4 KiB | 0 |
| kanna derive, `help` | 81 KiB | 82.6 KiB | 4 (build-time only) |
| clap builder, minimal (`std, help, usage, error-context`) | 199.0 KiB | 199.0 KiB | 3 |
| clap builder, default features (adds `color`, `suggestions`)¹ | 214.1 KiB | 214.1 KiB | 11 |
| clap derive, default features¹ | 225.1 KiB | 225.1 KiB | 19 |

¹ Measured once with the same clap version; only the minimal clap
variant is part of `benches/size.sh --compare`.

Between the two reports the `help` build grew by 0.9 KiB for the value
descriptions, the unsigned value type and the named missing argument of
ADR-0021; ADR-0022 added nothing to `kanna` itself. kanna with every
feature on is 0.47 of clap with the fewest features on, and 0.44 of clap
with its defaults; against a comparable feature set (kanna `full`
versus clap default) the saving is about 120 KiB per binary. The
`decl[help]` budget of 75 KiB (raised from 60 by ADR-0018) is now used
to the byte; the `full / clap < 0.5` gate has 0.03 to spare.

### Clean build time

| Variant | dev | release |
|---------|----:|--------:|
| kanna-core | 0.84 s | 1.10 s |
| kanna builder, `help` | 0.87 s | 1.27 s |
| kanna builder, `full` | 0.90 s | 1.35 s |
| kanna `cli!`, `full` | 0.92 s | 1.41 s |
| kanna derive, `help` | 2.52 s | 2.90 s |
| clap builder, minimal | 1.84 s | 2.72 s |
| clap builder, default features¹ | 1.94 s | 3.12 s |
| clap derive, default features¹ | 2.98 s | 3.16 s |

The builder and `cli!` fronts build about twice as fast as clap. The
derive front does not: both pull in `syn`, which dominates the build.

### Size of the code to audit

| Crate | Lines of Rust | `unsafe` |
|-------|--------------:|:--------:|
| kanna-core | 857 | forbidden |
| kanna (builder, `cli!`, help, suggest, color, env, json) | 5,793 | forbidden |
| kanna-derive | 635 | forbidden |
| kanna-complete + kanna-doc + kanna-schema | 3,282 | forbidden |
| kanna-prompt | 440 | forbidden |
| clap_lex | 810 | 5 blocks (`OsStr` splitting) |
| clap_builder | 28,992 | forbidden |
| clap_derive | 4,575 | forbidden |

kanna's parser (core + builder) is a little over one fifth the size of
clap's, and its `Cargo.toml` inherits `unsafe_code = "forbid"` for every
crate. The workspace runs 251 tests (unit, integration, snapshot and
doc tests, all features on) plus a 200,000-case pseudo-fuzz and a
libFuzzer target, on Linux, macOS and Windows in CI. Both projects have
an MSRV of 1.85.

## Where kanna is better

* **Binary size.** Two fifths of clap's with all comparable features on.
  For a tool shipped as a single static binary, or for a project with
  many small binaries, that is the reason to pick it.
* **Build time.** Clean builds of the builder and `cli!` fronts take
  under a second against clap's 1.8 to 1.9 s, and incremental rebuilds of
  the application only recompile one small crate. Parsing speed is
  negligible in both.
* **Zero dependencies unless you opt into derive.** `cargo tree` for a
  `full`-featured kanna binary shows nothing but `std`, and so does one
  that adds completion, documentation, schema or the REPL. clap's
  default configuration brings 11 crates, the derive one 19.
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
* **One definition, many outputs.** The same `Command` serves the
  parser, the completion generator (five shells, static and dynamic),
  the manpage / Markdown / HTML generator, the JSON description, the
  Claude / MCP tool definitions and, since 0.2.1, a REPL
  (`kanna-prompt`: a line at the prompt is parsed, checked and helped
  exactly like a command line). clap's equivalents are the separate
  `clap_complete` and `clap_mangen` crates; tool definitions and a REPL
  come from third parties or by hand.
* **Value descriptions in the definition.** A `///` comment on a value
  enum variant reaches `--help` (`[possible values: c = Celsius, ...]`)
  and the tool schema. clap has `PossibleValue::help` for the builder
  and `#[value(help)]` for derive, so this is parity rather than a lead,
  but in kanna the same text also feeds the agent-facing outputs.
* **Auditability.** About 6,650 lines for lexer and builder together, no
  `unsafe` anywhere.
* **Built for use with language models.** A one-page complete reference,
  compile errors that list the valid settings, verified examples, tool
  definitions with value legends and `no_tool`, and structured errors
  that name their kind and argument (table below). clap has none of
  these in the crate itself.

## Where clap is still better

* **Remaining feature gaps** (all judged rare, see ADR-0018): `Arg::last`
  (values only after `--`), `multicall` (busybox-style binaries),
  `value_hint` (kanna has dynamic completion callbacks instead),
  `default_value_if`, help templates, `next_line_help`, hidden possible
  values, `ignore_case` for values, `indices_of`, and terminal width
  *detection* (kanna reads `$COLUMNS` or takes `term_width`; clap
  queries the terminal).
* **Ecosystem and maturity.** clap has more than a decade of production
  use, tens of thousands of dependents, a tutorial, a cookbook, a FAQ,
  and companion crates. Bug reports and edge cases have been ironed out
  across every platform. kanna is at 0.2.1: seven crates on crates.io,
  CI on three platforms, but a single author, no known external users,
  and an API that may still change before 1.0 (0.2.0 already changed
  the return type of `to_argv` and the JSON document format).
* **Derive build time is not better.** Anyone choosing kanna for its
  derive front gets clap-like build times because `syn` dominates; only
  the size advantage remains.
* **The size margin is spent.** `decl[help]` sits at 75.0 KiB under a
  75 KiB budget and `full / clap` at 0.47 under 0.5; the next feature in
  the default build forces a budget decision, whereas clap has no such
  constraint to manage.
* **Documentation volume.** API docs, README, migration guides and ADRs
  exist, and six complete example programs, but there is no tutorial
  and far fewer worked examples than clap's cookbook.

## Working with a language model

Since ADR-0019 this is an axis of its own. What each library offers to
a model that writes a CLI, and to an agent that calls one:

| Concern | clap 4 | kanna 0.2.1 |
|---------|--------|-------------|
| A complete, single-file list of every setting | none in the crate; the docs are a large rustdoc tree, a tutorial and a cookbook | `docs/ai/kanna-reference.md` plus a Claude Code skill |
| Wrong setting name in a derive/macro | derive: compile error listing valid attributes; builder: a runtime `debug_assert` for some mistakes | derive and `cli!`: compile error naming the setting and listing every valid one |
| Definition mistakes | `Command::debug_assert()`, first problem only | `Command::validate()`, every problem, automatic in debug builds |
| Examples in the definition | free text in `after_help` | `example = ".."`, rendered in help and docs and parsed by `check_examples()` |
| Exposing the CLI as an agent tool | not in clap itself | `kanna_schema::tool`: Claude / MCP tool definitions with JSON Schema inputs (value legends from doc comments, `minimum: 0` for unsigned, `no_tool` to keep the driver's own commands out), and `to_argv` back to a validated command line |
| Machine-readable description of the CLI | not in clap itself | `kanna_schema::to_json` (format 3, with a JSON Schema) |
| Machine-readable errors | text only | `Error::to_json`, `Error::kind` / `Error::arg` (also from `to_argv` and for a single missing argument), `KANNA_ERROR_FORMAT=json` (feature `json`) |
| Suggestions for typos | `suggestions` feature | `suggest` feature (options, subcommands, values) |
| An interactive session over the same definition | third-party crates | `kanna-prompt`, no dependencies |

The size cost of the agent-facing additions is 3.2 KiB on the default
build (2.3 KiB from ADR-0019, 0.9 KiB from ADR-0021) and 5.6 KiB more
for `json`.

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
| `--unit C` for a value `c` | error unless `ignore_case` | error (no `ignore_case`; the help legend shows the spelling) |

## Recommendation

Pick kanna when binary size, dependency count or clean-build time is a
stated requirement and you are prepared to be an early adopter, or when
the CLI will be written by, or driven by, a language model and the
definition should also produce tool schemas, structured errors or a
REPL. The feature set covers what most command line tools need,
including flattened option structs, conditional requirements, value
enums with descriptions, help headings and external subcommands. Pick
clap when its ecosystem, stability and support community matter more
than roughly 120 KiB per binary, or when one of the remaining gaps
above is essential.

The two are close in API vocabulary (see `docs/migration/from-clap.md`),
so a program written against either can move to the other.
