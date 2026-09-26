# From clap

## Builder

| clap 4 | hasami |
|--------|--------|
| `Command::new("app")` | `Command::new("app")` |
| `.version(..)`, `.about(..)`, `.long_about(..)`, `.after_help(..)` | same names |
| `Arg::new("id").long("name").short('n')` | `Arg::new("name").short('n')` (id = long by default; `Arg::with_id("id").long("name")` to differ) |
| `.action(ArgAction::SetTrue)` | default: an `Arg` is a flag |
| `.action(ArgAction::Count)` | `.count()` |
| `.value_parser(value_parser!(u32))` | `.value::<u32>()` |
| `.value_parser(clap::builder::PossibleValuesParser::new([..]))` | `.value::<String>().possible([..])` |
| `.value_parser(value_parser!(OsString))` | `.value_os()` |
| `.default_value("1")` | `.default(1)` (typed) |
| `.num_args(0..=1).default_missing_value("x")` | `.default_missing(x)` (attached values only, see ADR-0009) |
| `.action(ArgAction::Append)` | `.many()` |
| `.required(true)` | `.required()` (the `Arg` type changes from `Option<T>` to `T`) |
| `.env("VAR")` | `.env("VAR")` with feature `env` |
| `.hide(true)` | `.hidden()` |
| `.global(true)` | `.global()` |
| `.alias("x")`, `.short_alias('x')` | same |
| `Arg::new("FILE").index(1)` | `Arg::positional::<T>("FILE")` (order of `.arg()` calls) |
| `.conflicts_with("b")` | `Command::exclusive([a.id(), b.id()])` |
| `.requires("b")` | `Command::requires(&a, &b)` |
| `ArgGroup::new("g").args([..]).required(true).multiple(false)` | `Group::new("g").member(&a).member(&b).required().exclusive()` |
| `.subcommand(Command::new("sub"))` | same; lazy with `Subcommand::lazy("sub", || ..)` or `subcommand_lazy` |
| `.subcommand_required(true)` | `.subcommand_required()` |
| `.disable_help_flag(true)` / `.disable_version_flag(true)` | `.disable_help()` / `.disable_version()` |
| `cmd.get_matches()` | `cmd.parse()` |
| `cmd.try_get_matches_from(iter)` | `cmd.try_parse_from(iter)` (first item is the binary name) or `try_parse_args(iter)` |
| `m.get_one::<T>("id")` | `m.get(&arg)` — the `Arg` is the key; returns `Option<T>` or `T` per definition |
| `m.get_flag("id")` | `m.get(&flag)` → `bool` |
| `m.get_count("id")` | `m.get(&counter)` → `usize` |
| `m.get_many::<T>("id")` | `m.get(&many_arg)` → `Vec<T>` |
| `m.contains_id("id")` | `m.contains(&arg)` / `m.contains_id("id")` |
| `m.value_source("id")` | `m.source(&arg)` |
| `m.subcommand()` | `m.subcommand()` → `Option<(&str, &Matches)>` |
| `Error::kind()` | `Error::kind()` (`ErrorKind::DisplayHelp` etc.) |
| `Error::exit()` | `Error::exit()` |

Behavioural differences worth knowing:

* Repeated single-value options: clap errors by default, hasami takes the
  last value (`occurrences()` still counts them).
* `--opt val` for an option with an optional value: clap may consume
  `val`; hasami never does (attached form only).
* Help layout follows clap 4, but long help strings are not wrapped to the
  terminal width.
* No `help` subcommand is added, only `-h/--help`.

## Derive

| clap | hasami |
|------|--------|
| `#[derive(Parser)]` | `#[derive(Args)]` + `use hasami::Cli` |
| `#[derive(Subcommand)]` | `#[derive(Commands)]` |
| `#[command(name, version, about, long_about, after_help)]` | `#[hasami(name = .., version = .., about = .., long_about = .., after_help = ..)]` |
| `#[arg(short, long)]` | `#[hasami(short)]` (long is implied) |
| `#[arg(default_value_t = 1)]` | `#[hasami(default = 1)]` |
| `#[arg(value_enum)]` | `#[hasami(possible = ["a", "b"])]` with a `FromStr` type |
| `#[arg(action = ArgAction::Count)]` on `u8` | `#[hasami(count)]` on `usize` |
| `#[arg(env = "X")]` | `#[hasami(env = "X")]` |
| `#[arg(index = 1)]` / bare positional | `#[hasami(positional)]` |
| `#[command(subcommand)]` | `#[hasami(subcommand)]` |
| `#[command(flatten)]` | not supported |
| `Cli::parse()` | `Cli::parse()` (trait method) |
