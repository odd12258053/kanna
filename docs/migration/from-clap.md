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
| `.num_args(1..)` (several values per occurrence) | `.many().greedy()` |
| `.value_delimiter(',')` | `.many().delimiter(',')` |
| `.args_override_self(true)` on the command / repeated single options | `Command::args_override_self()` or `Arg::last_wins()`; without them a repeat is `ErrorKind::Repeated` |
| `.value_parser(value_parser!(Level))` with `ValueEnum` | `.value_enum::<Level>()`, `Level` from `value_enum!` or `#[derive(ValueEnum)]` |
| `.long_help(..)`, `.help_heading(..)`, `.visible_alias(..)` | same names |
| `.requires("b")`, `.conflicts_with("b")`, `.required_unless_present("b")`, `.required_if_eq("b", "v")`, `.requires_if("v", "b")` | `Arg::requires(&b)`, `conflicts_with(&b)`, `required_unless(&b)`, `required_if_eq(&b, "v")`, `requires_if("v", &b)` |
| `.trailing_var_arg(true)` / `.raw(true)` on a positional | `Arg::positional_os("ARGS").many().trailing()` |
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
| `.before_help(..)`, `.long_version(..)`, `.term_width(n)` | same names |
| `.arg_required_else_help(true)` | `.arg_required_else_help()` (`ErrorKind::HelpOnMissingArgs`, status 2) |
| `.infer_long_args(true)`, `.infer_subcommands(true)` | `.infer_long_args()`, `.infer_subcommands()` |
| `.allow_external_subcommands(true)` + `ArgMatches::subcommand()` with `(name, external)` | `.allow_external_subcommands()` + `Matches::external_subcommand()` |
| `.styles(Styles::styled()...)` | `.styles(Styles::COLORED.with_header("..")...)` (feature `color`) |
| `Command::debug_assert()` | `Command::validate()`; parsing runs it in debug builds |
| `cmd.get_matches()` | `cmd.parse()` |
| `cmd.try_get_matches_from(iter)` | `cmd.try_parse_from(iter)` (first item is the binary name) or `try_parse_args(iter)` |
| `m.get_one::<T>("id")` | `m.get(&arg)` — the `Arg` is the key; returns `Option<T>` or `T` per definition |
| `m.get_flag("id")` | `m.get(&flag)` → `bool` |
| `m.get_count("id")` | `m.get(&counter)` → `usize` |
| `m.get_many::<T>("id")` | `m.get(&many_arg)` → `Vec<T>` |
| `m.contains_id("id")` | `m.contains(&arg)` / `m.contains_id("id")` |
| `m.value_source("id")` | `m.source(&arg)` |
| `m.ids()` | `m.ids()` |
| `m.subcommand()` | `m.subcommand()` → `Option<(&str, &Matches)>` |
| `Error::kind()` | `Error::kind()` (`ErrorKind::DisplayHelp` etc.) |
| `Error::exit()` | `Error::exit()` |

Behavioural differences worth knowing:

* Repeated single-value options and flags are errors in both; hasami's
  opt-out is `last_wins()` / `args_override_self()`.
* `--opt val` for an option with an optional value: clap may consume
  `val`; hasami never does (attached form only).
* `--opt -1` and `--opt --other`: hasami takes the next argument as the
  value (GNU style); clap needs `allow_hyphen_values`.
* Help wraps at `term_width`, `$COLUMNS` or 100 columns; clap detects the
  terminal width itself. `-h` and `--help` differ in both.
* `app help sub` works in both when the command has subcommands.
* Conditional requirements compare raw values and only count arguments
  given explicitly; a default value never triggers `required_if_eq`.

## Derive

| clap | hasami |
|------|--------|
| `#[derive(Parser)]` | `#[derive(Args)]` + `use hasami::Cli` |
| `#[derive(Subcommand)]` | `#[derive(Commands)]` |
| `#[command(name, version, about, long_about, after_help)]` | `#[hasami(name = .., version = .., about = .., long_about = .., after_help = ..)]` |
| `#[arg(short, long)]` | `#[hasami(short)]` (long is implied) |
| `#[arg(default_value_t = 1)]` | `#[hasami(default = 1)]` |
| `#[arg(value_enum)]` | `#[hasami(value_enum)]` with a `#[derive(ValueEnum)]` type (or `possible = [..]` with any `FromStr` type) |
| `#[arg(action = ArgAction::Count)]` on `u8` | `#[hasami(count)]` on `usize` |
| `#[arg(env = "X")]` | `#[hasami(env = "X")]` |
| `#[arg(index = 1)]` / bare positional | `#[hasami(positional)]` |
| `#[command(subcommand)]` | `#[hasami(subcommand)]` |
| `#[command(flatten)]` | `#[hasami(flatten)]` |
| `#[arg(long_help, help_heading, visible_alias, requires, conflicts_with, required_unless_present, required_if_eq, requires_if, num_args(1..), value_delimiter, trailing_var_arg)]` | `long_help`, `help_heading`, `visible_alias`, `requires`, `conflicts_with`, `required_unless`, `required_if_eq = ["f", "v"]`, `requires_if = ["v", "f"]`, `greedy`, `delimiter = ','`, `trailing` |
| `#[command(before_help, long_version, args_override_self, arg_required_else_help, infer_long_args, infer_subcommands, allow_external_subcommands, term_width)]` | same names inside `#[hasami(...)]` |
| `Cli::parse()` | `Cli::parse()` (trait method) |
