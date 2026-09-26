# hasami

A layered command line argument parser for Rust.

> lexopt's correctness and lightness at the core, with clap-grade
> ergonomics you can stack on top **only when you want them**.

| Layer | Crate / feature | What you get | Cost on the sample CLI¹ |
|-------|-----------------|--------------|------------------------:|
| 1. Lexer | `hasami-core` | GNU/POSIX-correct `Parser::next()` loop, `OsString` values, zero dependencies, one file, no `unsafe` | 13.5 KiB, 0.6 s build |
| 2. Builder | `hasami` | `Command` / `Arg<T>` with typed values, subcommands, constraints (groups, requires, conditional), value enums, `--help` with headings and wrapping, errors with tips, definition validation | 71.8 KiB, 0.8 s |
| 3. `cli!` | `hasami` | A struct-shaped DSL with `macro_rules!` only: doc comments become help, `#[flatten]`, `#[subcommand]` | 79.3 KiB, 0.8 s |
| 4. Derive | `hasami` + `derive` | `#[derive(Args)]`, `#[derive(Commands)]`, `#[derive(ValueEnum)]` with the same semantics as `cli!` | 79.2 KiB, 2.3 s |
| + | `hasami-complete`, `hasami-doc`, `hasami-schema` | Shell completion (5 shells, static and dynamic), manpage / Markdown / HTML, JSON description | separate crates |

¹ Binary size delta over an empty `main`, clean dev build time; full tables [below](#how-it-compares).

## Which layer?

```
Do you want a struct filled in for you?
├─ no  → Are you writing a tiny tool, or do you need order-dependent / dynamic options?
│        ├─ yes → hasami-core            (layer 1)
│        └─ no  → hasami::Command        (layer 2)
└─ yes → Is a 2 s build-time hit for syn acceptable?
         ├─ no  → hasami::cli!           (layer 3)
         └─ yes → #[derive(hasami::Args)] (layer 4)
```

Every layer lowers to the same `Command` value, so help text, error
messages, completions and schemas are identical whichever you pick, and
you can move between layers without changing behaviour.

## Layer 1: the lexer

```rust
use hasami_core::prelude::*;

fn main() -> Result<(), hasami_core::Error> {
    let mut number = 1u32;
    let mut shout = false;
    let mut thing: Option<String> = None;

    let mut parser = Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('n') | Long("number") => number = parser.value()?.parse()?,
            Long("shout") => shout = true,
            Value(v) => thing = Some(v.string()?),
            _ => return Err(arg.unexpected()),
        }
    }
    Ok(())
}
```

Handles `--opt=val`, `--opt val`, `-oval`, `-o val`, `-o=val`, `-abc`,
`--`, `-`, optional values (`--color[=WHEN]`), multi-values and raw tails;
never panics; passes non-Unicode bytes through untouched. Rules: [ADR-0003].

## Layer 2: the builder

```rust
use hasami::{Arg, Command};

let number = Arg::new("number").short('n').value::<u32>().default(1).help("How many times");
let shout = Arg::new("shout").help("Use upper case");
let thing = Arg::positional::<String>("THING").required().help("Whom to greet");

let cmd = Command::new("greet").version("0.1.0").about("Greet someone")
    .arg(&number).arg(&shout).arg(&thing);

let m = cmd.parse();            // prints help/errors and exits as needed
let n: u32 = m.get(&number);    // typed: the Arg is the key
```

The `Arg` you defined is the key you read with: no strings, no downcasts.
Values are parsed and validated before your code runs. Constraints:
`Command::exclusive`, `Command::requires`, `Group`, and per argument
`requires`, `conflicts_with`, `required_unless`, `required_if_eq`,
`requires_if`. Subcommands nest, may be lazy, can see `global()` options
of their parents, and may be inferred from a prefix or passed through as
external subcommands. Repeated values come one per occurrence
(`.many()`), several per occurrence (`.greedy()`) or split on a
delimiter. Value enums (`value_enum!`) give help and parsing one source
of truth. `Command::validate()` reports every mistake in a definition
and runs by itself in debug builds.

```text
$ greet --nubmer 2 world
error: unexpected argument '--nubmer' found

  tip: a similar argument exists: '--number'

Usage: greet [OPTIONS] <THING>

For more information, try '--help'.
```

## Layer 3: `cli!`

```rust
use hasami::Cli;

hasami::cli! {
    /// Greet someone
    #[name = "greet", version = "0.1.0"]
    struct Args {
        /// Name of the person
        #[short = 'n'] name: String,
        /// Number of times
        #[default = 1] count: u8,
        /// Use upper case
        shout: bool,
        #[subcommand] cmd: Option<Cmd>,
    }
    enum Cmd { Add(AddArgs), Remove }
    struct AddArgs { #[positional] thing: String }
}

let args = Args::parse();
```

Field types drive the mapping: `bool` is a flag, `Option<T>` optional,
`Vec<T>` repeatable, `T` required (or `default = ..`), `usize` with
`count` a counter. See the [`cli!` docs](hasami/src/macros.rs) for every
setting.

## Layer 4: derive

```rust
use hasami::{Args, Cli};

/// Greet someone
#[derive(Args)]
#[hasami(name = "greet", version = "0.1.0")]
struct Greet {
    /// Name of the person
    #[hasami(short = 'n')]
    name: String,
    /// Number of times
    #[hasami(default = 1)]
    count: u8,
}

let args = Greet::parse();
```

Same vocabulary as `cli!`, inside `#[hasami(...)]`. The two produce
byte-identical binaries for the sample CLI.

## Extras

```rust
let cmd = Greet::command();
hasami_complete::dynamic::complete_from_env(&cmd);          // answer shell requests
let bash = hasami_complete::generate(hasami_complete::Shell::Bash, &cmd);
let man = hasami_doc::manpage(&cmd);
let json = hasami_schema::to_json(&cmd);                   // everything the parser accepts
```

`hasami/examples/extras.rs` shows all three; `hasami/examples/{core,builder,macro,derive}.rs` show each layer.
More in the same directory: `values` (typed values, custom parsers, `--color[=WHEN]`,
raw `OsString` paths), `nested` (nested subcommands, global options, lazy and
hidden subcommands), `errors` (`try_parse`, groups, `requires`, custom errors),
`env` (environment fallback, needs `--features env`) and `wrapper` (the core
lexer with `--` pass-through, multi-value options and error recovery).

Complete applications, one per layer, live under [`examples/`](examples/README.md):
a `wc` on the bare lexer, a grep with the builder, a todo list with `cli!` and a
`hexdump` with derive. Run them with `cargo run -p hasami-example-<name>`.

## Features of `hasami`

| Feature | Default | Adds |
|---------|:-------:|------|
| `help` | yes | `-h/--help`, `-V/--version`, help rendering |
| `std` | yes | reserved; the core needs `std` today ([ADR-0002]) |
| `suggest` | | "a similar argument exists" tips (Jaro-Winkler, no deps) |
| `color` | | ANSI colour honouring `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, TTY |
| `env` | | `Arg::env("VAR")` fallback |
| `derive` | | `#[derive(Args)]`, `#[derive(Commands)]` (pulls in syn) |
| `full` | | `help + suggest + color + env` |

With no features enabled `hasami` has **zero** third-party dependencies;
CI enforces it.

## How it compares

Same five-option, one-positional CLI written with each library; numbers
from `benches/size.sh --compare` and `benches/build-time.sh --compare` on
a 24-core x86_64 Linux box with rustc 1.98.1 (2026-09-26). Size is the
delta over an empty `main` with `opt-level="s"`, LTO, `panic="abort"`,
stripped ([ADR-0007]).

| Library | Style | Size overhead | Build (dev / release) | Deps | Invalid UTF-8 |
|---------|-------|--------------:|----------------------:|-----:|:-------------:|
| pico-args | imperative | 11.5 KiB | 0.76 s / 1.09 s | 1 | yes |
| **hasami-core** | imperative | **13.5 KiB** | **0.76 s / 1.06 s** | **0** | yes |
| argh | derive | 15.7 KiB | 3.31 s / 3.46 s | 13 | no |
| lexopt | imperative | 16.6 KiB | 0.78 s / 1.07 s | 1 | yes |
| **hasami** (builder, `help`) | builder | **71.8 KiB** | **0.86 s / 1.31 s** | **0** | yes |
| **hasami** (`cli!`, `help`) | macro DSL | 79.3 KiB | 0.87 s / 1.38 s | 0 | yes |
| **hasami** (derive, `help`) | derive | 79.2 KiB | 2.3 s / 2.5 s | 4 | yes |
| **hasami** (builder, `full`) | builder | 87.6 KiB | 0.83 s / 1.40 s | 0 | yes |
| bpaf | combinators | 95.1 KiB | 0.78 s / 1.25 s | 1 | yes |
| clap 4 | builder | 199.0 KiB | 1.79 s / 2.73 s | 4 | yes |

Budgets enforced in CI: core ≤ 20 KiB, builder+help ≤ 75 KiB, builder
with every feature < ½ of clap, core clean build ≤ 2 s, full ≤ 4 s.

## Design principles

1. **Correctness first**: GNU/POSIX conventions in full, pinned by 66
   lexer tests, a 200 000-case pseudo-fuzz on stable and a libFuzzer target.
2. **Non-Unicode safe**: `OsString` throughout; conversion is explicit and
   fallible; no panics on any input.
3. **Zero-dependency core**: one auditable file, `#![forbid(unsafe_code)]`.
4. **Pay for what you use**: everything beyond lexing is a feature flag.
5. **Build time**: `syn` only behind `derive`.
6. **Errors that help**: what, where, and how to fix it.
7. **Stability**: MSRV 1.85; every decision is an [ADR](docs/ADR/README.md).

Non-goals: a clap compatibility shim, TUI/prompt/progress features,
Windows `/opt` syntax.

## Documentation

* [Architecture decision records](docs/ADR/README.md)
* [Migration guides](docs/migration/README.md) from clap, argh, bpaf,
  lexopt, pico-args and xflags
* [Step reports](docs/reports/) with measurements
* [CHANGELOG](CHANGELOG.md)

## License

MIT or Apache-2.0, at your option.

[ADR-0002]: docs/ADR/0002-core-requires-std.md
[ADR-0003]: docs/ADR/0003-lexing-rules.md
[ADR-0007]: docs/ADR/0007-size-measurement.md
