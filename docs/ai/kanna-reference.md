# kanna: the complete reference on one page

This file is written for people and language models that generate code
against kanna. It lists **every** public name and setting, in the exact
spelling, with the rules the parser follows. If a name is not here, it
does not exist; in particular none of clap's `#[arg]`, `#[command]`,
`ArgAction`, `value_parser!`, `get_one`, `.index()`, `.num_args()`,
`.action()` exist in kanna (the mapping is at the end).

## 1. Pick a layer and the features

| Need | Use | `Cargo.toml` |
|------|-----|--------------|
| A tiny tool, order-dependent options, no help text | `kanna-core` (a lexer loop) | `kanna-core = "0.2"` |
| Typed options, subcommands, constraints, `--help` | `kanna` builder | `kanna = "0.2"` (default features: `help`, `std`) |
| A struct filled in, no proc-macro | `kanna::cli!` | same |
| A struct filled in, `#[derive]` | `kanna::Args` etc. | `kanna = { version = "0.2", features = ["derive"] }` |
| Shell completion, man/Markdown/HTML, JSON schema, AI tool definitions | `kanna-complete`, `kanna-doc`, `kanna-schema` | separate crates, each depends on `kanna` |

Features of `kanna`: `help` (default; `-h/--help`, `-V/--version`),
`suggest` ("did you mean" tips), `color` (ANSI, honours `NO_COLOR`,
`CLICOLOR`, `CLICOLOR_FORCE`, TTY), `env` (`Arg::env` fallback), `json`
(`Error::to_json`, `KANNA_ERROR_FORMAT=json`), `derive`, `full` (= help +
suggest + color + env + json). With no features, `kanna` has zero
third-party dependencies. MSRV 1.85, edition 2024.

Everything below is available in all three front ends; the builder
method, the `cli!` setting and the `#[kanna(...)]` setting share a name.

## 2. The builder (`kanna::{Arg, Command, Matches, Error}`)

### 2.1 Defining arguments: `Arg<T>` is a type-state builder

`T` is what `Matches::get(&arg)` returns.

| Start | Method | Result type | Meaning |
|-------|--------|-------------|---------|
| `Arg::new("name")` | | `Arg<bool>` | flag `--name`; id = `name` |
| `Arg::with_id("id")` | then `.long("name")` / `.short('n')` | `Arg<bool>` | flag with explicit names |
| `Arg::positional::<T>("NAME")` | | `Arg<Option<T>>` | positional, `T: FromStr`, placeholder `<NAME>` |
| `Arg::positional_with("NAME", f)` | | `Arg<Option<T>>` | positional parsed by `f: Fn(&str) -> Result<T, E>` |
| `Arg::positional_os("NAME")` / `positional_os_with` | | `Arg<Option<OsString>>` | raw bytes, never fails on non-UTF-8 |
| `Arg::positional_enum::<E>("NAME")` | | `Arg<Option<E>>` | `E: ValueEnum` |
| `Arg<bool>` | `.count()` | `Arg<usize>` | `-vvv` → 3 |
| `Arg<bool>` | `.value::<T>()` | `Arg<Option<T>>` | takes a value, `T: FromStr` (error type must be `Display`) |
| `Arg<bool>` | `.value_with(f)` / `.value_os()` / `.value_os_with(f)` | `Arg<Option<T>>` | custom parser / raw `OsString` |
| `Arg<bool>` | `.value_enum::<E>()` | `Arg<Option<E>>` | `E: ValueEnum`; possible values come from `E` |
| `Arg<Option<T>>` | `.required()` | `Arg<T>` | must be given |
| `Arg<Option<T>>` | `.default(v)` | `Arg<T>` | `v: Display` shown in help |
| `Arg<Option<T>>` | `.default_with(v, "text")` | `Arg<T>` | for `T` without `Display` |
| `Arg<Option<T>>` | `.default_missing(v)` / `default_missing_with(v, "text")` | `Arg<Option<T>>` | optional value `--color[=WHEN]`: `--color` alone gives `v`; value must be attached (`--color=never`, `-cnever`); `--color never` treats `never` as a positional. Call before `required`/`default`/`many` |
| `Arg<Option<T>>` | `.possible(["a", "b"])` | `Arg<Option<T>>` | allowed spellings, checked before parsing, listed in help |
| `Arg<Option<T>>` | `.possible_with_help([("a", "Plan A"), ("b", "Plan B")])` | `Arg<Option<T>>` | `possible` plus a description per value: shown in `--help` and in tool definitions |
| `Arg<Option<T>>` | `.complete_with(\|prefix\| vec![..])` | `Arg<Option<T>>` | dynamic completion candidates |
| `Arg<Option<T>>` | `.env("VAR")` | `Arg<Option<T>>` | feature `env`; used when absent, before the default, validated like a value |
| `Arg<Option<T>>` | `.many()` | `Arg<Vec<T>>` | repeatable, one value per occurrence; for a positional: all remaining positionals |
| `Arg<Vec<T>>` | `.required()` | `Arg<Vec<T>>` | at least one |
| `Arg<Vec<T>>` | `.greedy()` | `Arg<Vec<T>>` | each occurrence takes every following non-option word: `--exec cmd a b` |
| `Arg<Vec<T>>` | `.delimiter(',')` | `Arg<Vec<T>>` | `-f a,b,c` → three values |
| `Arg<Vec<T>>` | `.trailing()` | `Arg<Vec<T>>` | positional only: after its first value, everything (even `-x`) is a value |

Methods on any `Arg<T>`: `.long("name")`, `.short('c')`, `.alias("n")`,
`.short_alias('c')`, `.visible_alias("n")` (shown as `[aliases: n]`),
`.help("text")`, `.long_help("text")` (shown by `--help`, `help` by `-h`),
`.help_heading("Title")` (own section in help), `.hidden()`, `.global()`
(visible in all subcommands below), `.value_name("NAME")`,
`.last_wins()` (repeats replace instead of erroring),
`.requires(&other)`, `.conflicts_with(&other)`, `.required_unless(&other)`,
`.required_if_eq(&other, "value")`, `.requires_if("value", &other)`.
Getters: `.id()`, `.def()` → `&ArgDef` (read-only view, e.g.
`def().value_type()`).

### 2.2 Defining commands: `Command`

```rust
let cmd = Command::new("app")
    .version("1.0")                 // enables -V/--version
    .long_version("1.0 (build 7)")  // printed by --version only
    .about("One line")              // top of -h, and the parent's subcommand list
    .long_about("Longer text")      // top of --help
    .before_help("Banner")          // very first thing in help
    .after_help("Footer")           // last thing in help
    .example("app add cake --sweet") // repeatable; shown under Examples:, verified by check_examples()
    .arg(&a).arg(&b)                // order = positional order = help order
    .subcommand(Command::new("add").about("...").arg(&x))
    .subcommand(Subcommand::lazy("rm", || Command::new("rm")).about("...").alias("remove").visible_alias("delete").hidden())
    .subcommand_lazy("ls", "List", || Command::new("ls"))
    .subcommand_required()
    .group(Group::new("fmt").member(&json).member(&yaml).required().exclusive())
    .exclusive([a.id(), b.id()])    // shorthand: an exclusive group
    .requires(&a, &b)               // if a is present, b must be
    .args_override_self()           // every arg may repeat, last wins
    .arg_required_else_help()       // no args at all → help on stderr, exit 2
    .infer_long_args()              // --verb for --verbose when unique
    .infer_subcommands()            // inst for install when unique
    .allow_external_subcommands()   // unknown first word → Matches::external_subcommand()
    .disable_help().disable_version()
    .term_width(80)                 // help wrap width; default $COLUMNS or 100; 0 = no wrap
    .styles(Styles::COLORED.with_header("\x1b[35m")); // feature color
```

Getters mirror the setters: `name()`, `get_version()`, `get_about()`,
`get_long_about()`, `get_after_help()`, `get_before_help()`,
`get_long_version()`, `get_examples()`, `args()`, `subcommands()`,
`groups()`, `requirements()`, `find_arg(id)`, `find_subcommand(name)`,
`has_help_flag()`, `has_version_flag()`, `is_subcommand_required()`,
`is_args_override_self()`, `is_arg_required_else_help()`,
`is_infer_long_args()`, `is_infer_subcommands()`,
`allows_external_subcommands()`, `get_term_width()`, `get_styles()`.

Checks: `cmd.validate() -> Result<(), Vec<String>>` (every rule about a
well-formed definition; parsing runs it in debug builds and panics with
the list), `cmd.check_examples() -> Result<(), Vec<String>>` (parses
each example; failures as `` `example`: message ``).

Rendering (feature `help`): `cmd.render_help()` (`--help` form),
`cmd.render_short_help()` (`-h` form), `cmd.render_usage()`.

### 2.3 Parsing and reading values

```rust
let m: Matches = cmd.parse();                    // exits on error: help/version → stdout, status 0; errors → stderr, status 2
let r: Result<Matches, Error> = cmd.try_parse(); // same, but returns
cmd.parse_from(["app", "--x"]); cmd.try_parse_from(iter)   // first item is the binary name
cmd.try_parse_args(["--x"])                                // no binary name (tests)
cmd.try_parse_with(&mut kanna_core::Parser)               // reuse a core parser

m.get(&arg)            // T: bool | usize | Option<T> | T | Vec<T>, by the Arg's type. Panics if arg is not from this command
m.try_get(&arg)        // Option<T>
m.contains(&arg) / m.contains_id("id")   // given explicitly (command line or env); defaults do not count
m.occurrences(&arg)    // times on the command line
m.raw(&arg) / m.raw_id("id")             // Vec<&OsStr>, unparsed
m.source(&arg)         // Some(Source::CommandLine | Env | Default)
m.ids()                // iterator of ids given explicitly
m.subcommand()         // Option<(&str, &Matches)>; nested levels have their own Matches
m.subcommand_name(); m.subcommand_matches("add")
m.external_subcommand() // Option<(&OsStr, &[OsString])>
```

In a subcommand's `Matches`, read that level's arguments; global
arguments of a parent are read from the parent's `Matches` (they may be
written anywhere on the line).

### 2.4 Errors

`Error` (`Display` renders `error: <message>`, optional `tip:`, usage,
"For more information, try '--help'"), `e.kind() -> ErrorKind`,
`e.message()`, `e.tip()`, `e.usage()`, `e.arg()` (id the error is about),
`e.is_display()` (help/version), `e.exit_code()` (0 or 2), `e.print()`,
`e.exit() -> !`, `e.to_json()` (feature `json`). Constructors for
application errors: `Error::custom("text")`, `.with_tip(..)`,
`.with_usage(..)`, `.with_arg(..)`, `.with_source(err)`; `From<&str>`,
`From<String>`, `From<io::Error>`, `From<kanna_core::Error>`.

`ErrorKind` (`#[non_exhaustive]`, `.name()` gives the snake_case string):
`UnknownOption`, `UnknownSubcommand`, `UnexpectedArgument`,
`MissingValue`, `UnexpectedValue`, `InvalidValue`, `MissingRequired`,
`MissingSubcommand`, `Conflict`, `Repeated`, `HelpOnMissingArgs`,
`NonUnicode`, `DisplayHelp`, `DisplayVersion`, `Io`, `Custom`.

Setting `KANNA_ERROR_FORMAT=json` (feature `json`) makes `print()`/
`exit()` write one JSON object to stderr instead of text:
`{"kind":"invalid_value","exit_code":2,"arg":"number","message":"...","tip":null,"usage":"..."}`.

### 2.5 Value enums

```rust
kanna::value_enum! {
    /// doc comment allowed
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]   // Clone is required
    pub enum Level {
        /// Whisper                                 // variant doc = help(): shown in --help and tool definitions
        Low = "low",
        High = "high",                              // explicit names
    }
}
// generates: impl ValueEnum (VALUES, name(), help(), from_name(), names()), FromStr (Err = String), Display
```

With `derive`: `#[derive(kanna::ValueEnum, Clone)] enum Level { /// Whisper
Low, #[kanna(name = "very-high")] High }` — names default to kebab-case,
doc comments are the help.

## 3. `cli!` (no proc-macro) and `#[derive(Args)]`

```rust
kanna::cli! {
    /// About text (doc comment)
    #[derive(Debug)]                              // ordinary derives pass through
    #[name = "app", version = "1.0"]              // command settings, comma separated, any number of #[...]
    struct Args {
        /// Help text for --name (doc comment)
        #[short, long = "user-name"] name: String,      // required (plain T)
        /// Optional
        count: Option<u32>,                             // --count <COUNT>
        /// Repeatable
        #[short = 'I', greedy] include: Vec<String>,    // -I a b
        /// Flag
        verbose: bool,
        /// Counter
        #[short, count] debug: usize,
        /// Default
        #[default = 8080] port: u16,
        #[positional] file: String,                     // <FILE>, required
        #[positional, trailing] rest: Vec<String>,      // everything after
        #[value_enum, default = Level::Low] level: Level,
        #[flatten] common: Common,                      // another cli!/derive struct
        #[subcommand] cmd: Option<Cmd>,                 // or `cmd: Cmd` = required
    }
    #[derive(Debug)]
    enum Cmd {
        /// Summary
        Add(AddArgs),                                   // payload = a struct from cli!/derive
        #[name = "rm", alias = "remove", visible_alias = "delete", hidden]
        Remove,                                         // unit variant
    }
    #[derive(Debug)]
    struct AddArgs { #[positional] thing: String }
    #[derive(Debug)]
    struct Common { #[short, global] quiet: bool }
}
use kanna::Cli;               // brings parse/try_parse/try_parse_args/command/from_matches
let args = Args::parse();
let cmd: kanna::Command = Args::command();   // the same Command the builder would make
```

Field type → argument kind: `bool` flag; `usize` value, or counter with
`count`; `Option<T>` optional value; `Vec<T>` repeatable; `T` required
value or `default = expr`. `T: FromStr + Clone + Send + Sync + 'static`
with a `Display` error. The long name defaults to the kebab-case field
name; the field name is the id.

Field settings (bare or `= value`): `long`, `long = "name"`, `short`,
`short = 'c'`, `alias = "n"`, `short_alias = 'c'`, `visible_alias = "n"`,
`hidden`, `global`, `positional`, `count`, `required` (for `Vec`),
`value_name = "NAME"`, `default = expr`, `default_missing = expr`,
`env = "VAR"`, `possible = ["a", "b"]`, `value_enum`, `help = "text"`,
`long_help = "text"`, `help_heading = "Title"`, `last_wins`, `greedy`,
`trailing`, `delimiter = ','`, `requires = "field"`,
`conflicts_with = "field"`, `required_unless = "field"`,
`required_if_eq = ["field", "value"]`, `requires_if = ["value", "field"]`,
`subcommand`, `flatten`. References name **fields** (ids), not long names.

Command settings: `name = expr`, `version = expr`, `long_version = expr`,
`about = expr`, `long_about = expr`, `before_help = expr`,
`after_help = expr`, `example = expr` (repeatable), `disable_help`,
`disable_version`, `args_override_self`, `arg_required_else_help`,
`infer_long_args`, `infer_subcommands`, `allow_external_subcommands`,
`term_width = expr`.

Variant settings: `name = "x"`, `alias = "x"`, `visible_alias = "x"`,
`hidden`, `no_tool` (in help, but not an agent tool: for `tools`,
`call`, `setup`-style commands). The variant doc comment is the summary;
without one the payload struct's doc is used. Builder equivalent:
`Subcommand::from(cmd).no_tool()`.

`#[derive(Args)]` (feature `derive`) takes the same settings inside
`#[kanna(...)]` on the struct, fields and (with `#[derive(Commands)]`)
variants; `#[kanna(subcommand)]` and `#[kanna(flatten)]` mark those
fields. An unknown setting in either front end is a compile error that
lists the valid ones.

## 4. `kanna-core` (the lexer)

```rust
use kanna_core::prelude::*;          // Parser, Arg, Short/Long/Value, ValueExt
let mut p = Parser::from_env();       // or from_iter(args with bin name) / from_args(without)
while let Some(arg) = p.next()? {     // Result<Option<Arg>, Error>
    match arg {
        Short('n') | Long("number") => n = p.value()?.parse()?,   // value: OsString; parse via ValueExt
        Long("color") => match p.optional_value()? { .. }         // attached value only
        Long("exec") => for v in p.values()? { .. }               // following non-option words
        Value(v) => { let s = v.string()?; .. }                   // positional; v: OsString (also every word after `--`)
        other => return Err(other.unexpected()),
    }
}
let rest = p.raw_args()?;             // take the remaining words untouched (RawArgs: peek, next_if, as_slice)
p.bin_name()                          // Option<&str>
```

Lexing: `--opt`, `--opt=val`, `--opt val`, `-o`, `-oval`, `-o=val`,
`-abc` (cluster), `-` is a value, `--` ends options. Errors:
`MissingValue`, `UnexpectedOption`, `UnexpectedArgument`,
`UnexpectedValue` (`--flag=x`), `NonUnicodeValue`, `ParsingFailed`,
`Custom`. Every error leaves the parser usable, so several can be
collected.

## 5. Parsing rules the builder follows

* Repeating a single-value option or a flag is an error (`Repeated`)
  unless `count()`, `many()`, `last_wins()` or `args_override_self()`.
* `--opt -1` and `--opt --other` take the next word as the value (GNU).
* Optional values (`default_missing`) must be attached: `--color=never`.
* Options after a subcommand resolve in the subcommand first, then in
  enclosing commands but only for `global()` arguments.
* Precedence: command line, then `env`, then `default`. Only the first
  two count as "present" for `contains`, groups and relations.
* Missing required arguments are reported together. Conditional
  requirements compare raw values of explicitly given arguments.
* `-h/--help`, `-V/--version` are synthetic and lose to a user argument
  with the same name. `app help sub` shows `sub`'s help.
* Help: `Usage:` line, `Commands:`, `Arguments:`, `Options:`, custom
  headings, `Examples:`, `after_help`; annotations `[default: x]`,
  `[possible values: a, b]`, `[env: VAR]`, `[aliases: n]`; wrapped at
  `term_width` / `$COLUMNS` / 100.
* Exit status: 0 for help/version, 2 for any usage error.
* Values are `OsString` end to end; `FromStr` parsers reject non-UTF-8
  with `NonUnicode`; `value_os`/`positional_os` never do.

## 6. Generators (separate crates)

```rust
let cmd = Args::command();                                  // or the builder's Command
kanna_complete::dynamic::complete_from_env(&cmd);           // first thing in main: answers shell completion requests
kanna_complete::generate(kanna_complete::Shell::Bash, &cmd) // static script; Zsh, Fish, PowerShell, Nushell
kanna_complete::generate_dynamic(shell, "bin")              // script that calls the binary back
kanna_doc::manpage(&cmd); kanna_doc::markdown(&cmd); kanna_doc::html(&cmd)
kanna_schema::to_json(&cmd); kanna_schema::to_json_pretty(&cmd) // JSON description, format 3, JSON_SCHEMA validates it
kanna_schema::tool::tools(&cmd)                             // Vec<Tool { name, path, description, input_schema }>: one per runnable command
tool.to_json() / tool.to_mcp_json(); kanna_schema::tool::to_json(&tools, mcp: bool)
kanna_schema::tool::to_argv(&cmd, "app_add", r#"{"count": 2}"#) // JSON tool input → Result<Vec<OsString>, kanna::Error>, verified by parsing
```

Tool input schemas use the argument ids as property names: flags are
booleans, counters and unsigned integers integers with `minimum: 0`,
values typed by `ValueType` (`Integer`, `Unsigned`, `Float`, `Boolean`,
`String`, `Path`, `Other`), `possible` → `enum` (with the values'
descriptions appended to the property description as
`[possible values: c = Celsius, ...]`), `many` → array, `required` →
required; the description is `long_about` else `about`, so put the
output format in `long_about`. Hidden arguments are omitted; `hidden`
and `no_tool` subcommands produce no tool; global options of parents are
included. `to_argv` errors are `kanna::Error`s: `kind()` and `arg()` are
the parser's own for a command line that does not parse,
`UnexpectedArgument` / `InvalidValue` with the key as `arg` for a bad
key or value shape, `Custom` for malformed JSON or an unknown tool; print
them with `e.exit()` like any other.

## 7. Testing recipe

```rust
#[test]
fn definition_is_sound() {
    let cmd = Args::command();
    cmd.validate().unwrap();          // also runs automatically in debug parses
    cmd.check_examples().unwrap();    // every `example` parses
}
#[test]
fn parses() {
    let a = Args::try_parse_args(["--name", "bob", "-vv", "file"]).unwrap();
    assert_eq!(a.debug, 2);
    let e = Args::try_parse_args(["--nmae", "x"]).unwrap_err();
    assert_eq!(e.kind(), kanna::ErrorKind::UnknownOption);
}
#[test]
fn help_snapshot() { insta_or_plain_assert!(Args::command().render_help()); }
```

For end-to-end snapshots use `trycmd` on an example binary (see
`kanna/tests/snapshots.rs`).

## 8. Coming from clap: what to write instead

| clap | kanna |
|------|--------|
| `#[derive(Parser)]` | `#[derive(Args)]` + `use kanna::Cli` |
| `#[derive(Subcommand)]` | `#[derive(Commands)]` |
| `#[command(name, version, about, ...)]` | `#[kanna(name = .., version = .., about = ..)]` |
| `#[arg(short, long)]` | `#[kanna(short)]` (long is implied) |
| `#[arg(default_value_t = 1)]` / `default_value = "x"` | `#[kanna(default = 1)]` (typed expression) |
| `#[arg(value_enum)]` + `#[derive(ValueEnum)]` | same names |
| `#[arg(action = ArgAction::Count)]` on `u8` | `#[kanna(count)]` on `usize` |
| `#[arg(action = ArgAction::Append)]` | `Vec<T>` field |
| `#[arg(num_args = 1..)]` / `value_delimiter = ','` | `greedy` / `delimiter = ','` |
| `#[arg(index = 1)]` | `#[kanna(positional)]` (order of fields) |
| `#[command(flatten)]` / `#[command(subcommand)]` | `#[kanna(flatten)]` / `#[kanna(subcommand)]` |
| `Arg::new("id").long("name").short('n').action(SetTrue)` | `Arg::new("name").short('n')` |
| `.value_parser(value_parser!(u32))` | `.value::<u32>()` |
| `.value_parser(["a", "b"])` | `.value::<String>().possible(["a", "b"])` |
| `.default_value("1")` | `.default(1)` |
| `.num_args(0..=1).default_missing_value("x")` | `.default_missing(x)` |
| `.required(true)` | `.required()` (changes the `Arg` type) |
| `.conflicts_with("b")` / `.requires("b")` | `.conflicts_with(&b)` / `.requires(&b)` (or `Command::exclusive`, `Command::requires`) |
| `ArgGroup::new("g").args([..]).required(true).multiple(false)` | `Group::new("g").member(&a).member(&b).required().exclusive()` |
| `cmd.get_matches()` / `try_get_matches_from` | `cmd.parse()` / `try_parse_from` / `try_parse_args` |
| `m.get_one::<T>("id")` / `get_flag` / `get_count` / `get_many` | `m.get(&arg)` (typed by the `Arg`) |
| `m.value_source("id")` | `m.source(&arg)` |
| `Command::debug_assert()` | `cmd.validate()` |
| `clap_complete::generate` / `clap_mangen` | `kanna_complete::generate` / `kanna_doc::manpage` |
