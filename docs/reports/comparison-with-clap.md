# hasami compared with clap 4

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
| hasami-core (imperative) | 13.5 KiB | 13.5 KiB | 0 |
| hasami builder, no features | 47.5 KiB | 57.1 KiB | 0 |
| hasami builder, `help` | 58.1 KiB | 71.8 KiB | 0 |
| hasami builder, `help + suggest` | 63.6 KiB | 77.7 KiB | 0 |
| hasami builder, `full` (help, suggest, color, env) | 72.3 KiB | 87.6 KiB | 0 |
| hasami `cli!`, `help` | 63.7 KiB | 79.3 KiB | 0 |
| hasami `cli!`, `full` | 78.1 KiB | 94.8 KiB | 0 |
| hasami derive, `help` | 63.7 KiB | 79.2 KiB | 4 (build-time only) |
| clap builder, minimal (`std, help, usage, error-context`) | 199.0 KiB | 199.0 KiB | 3 |
| clap builder, default features (adds `color`, `suggestions`) | 214.1 KiB | 214.1 KiB | 11 |
| clap derive, default features | 225.1 KiB | 225.1 KiB | 19 |

The parity features cost 13.7 KiB on the `help` build (ADR-0018 has the
breakdown). hasami with every feature on is now 0.44 of clap with the
fewest features on, and 0.41 of clap with its defaults; against a
comparable feature set (hasami `full` versus clap default) the saving is
about 126 KiB per binary. The `decl[help]` budget was raised from 60 to
75 KiB to admit this; the `full / clap < 0.5` gate is unchanged.

### Clean build time

| Variant | dev | release |
|---------|----:|--------:|
| hasami-core | 0.76 s | 1.06 s |
| hasami builder, `help` | 0.86 s | 1.31 s |
| hasami builder, `full` | 0.83 s | 1.40 s |
| hasami `cli!`, `full` | 0.87 s | 1.38 s |
| hasami derive, `help` (from README) | 2.3 s | 2.5 s |
| clap builder, minimal | 1.79 s | 2.73 s |
| clap builder, default features | 1.94 s | 3.12 s |
| clap derive, default features | 2.98 s | 3.16 s |

The builder and `cli!` fronts still build about twice as fast as clap.
The derive front does not: both pull in `syn`, which dominates the build.

### Size of the code to audit

| Crate | Lines of Rust | `unsafe` |
|-------|--------------:|:--------:|
| hasami-core | 857 | forbidden |
| hasami (builder, `cli!`, help, suggest, color, env) | 5,255 | forbidden |
| hasami-derive | 618 | forbidden |
| hasami-complete + hasami-doc + hasami-schema | 1,750 | forbidden |
| clap_lex | 810 | 5 blocks (`OsStr` splitting) |
| clap_builder | 28,992 | forbidden |
| clap_derive | 4,575 | forbidden |

hasami's parser (core + builder) is about one fifth the size of clap's,
and its `Cargo.toml` inherits `unsafe_code = "forbid"` for every crate.
Both projects have an MSRV of 1.85.

## Where hasami is better

* **Binary size.** Two fifths of clap's with all comparable features on.
  For a tool shipped as a single static binary, or for a project with
  many small binaries, that is the reason to pick it.
* **Build time.** Clean builds of the builder and `cli!` fronts take
  under a second against clap's 1.7 to 1.9 s, and incremental rebuilds of
  the application only recompile one small crate. Parsing speed is
  negligible in both.
* **Zero dependencies unless you opt into derive.** `cargo tree` for a
  `full`-featured hasami binary shows nothing but `std`. clap's default
  configuration brings 11 crates, the derive one 19.
* **A layered choice instead of one size.** Programs that just need a
  lexer take `hasami-core` at 13.5 KiB and grow into the builder without
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
* **Auditability.** About 6,100 lines for lexer and builder together, no
  `unsafe` anywhere, 208 tests plus a 200,000-case pseudo-fuzz and a
  libFuzzer target.

## Where clap is still better

* **Remaining feature gaps** (all judged rare, see ADR-0018): `Arg::last`
  (values only after `--`), `multicall` (busybox-style binaries),
  `value_hint` (hasami has dynamic completion callbacks instead),
  `default_value_if`, help templates, `next_line_help`, hidden possible
  values, `indices_of`, and terminal width *detection* (hasami reads
  `$COLUMNS` or takes `term_width`; clap queries the terminal).
* **Ecosystem and maturity.** clap has more than a decade of production
  use, tens of thousands of dependents, a tutorial, a cookbook, a FAQ,
  and companion crates. Bug reports and edge cases have been ironed out
  across every platform. hasami is at 0.1.0 with a single author, no
  published release, no external users, CI that has not yet run on
  Windows or macOS, and an API that may still change before 1.0.
* **Derive build time is not better.** Anyone choosing hasami for its
  derive front gets clap-like build times because `syn` dominates; only
  the size advantage remains.
* **Documentation volume.** API docs, README, migration guides and ADRs
  exist, but there is no tutorial and far fewer worked examples.

## Behavioural differences worth knowing

| Input | clap default | hasami |
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

Pick hasami when binary size, dependency count or clean-build time is a
stated requirement and you are prepared to be an early adopter. The
feature set now covers what most command line tools need, including
flattened option structs, conditional requirements, value enums, help
headings and external subcommands. Pick clap when its ecosystem,
stability and support community matter more than roughly 130 KiB per
binary, or when one of the remaining gaps above is essential.

The two are close in API vocabulary (see `docs/migration/from-clap.md`),
so a program written against either can move to the other.
