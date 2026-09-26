# From argh

argh's derive maps almost one to one onto `hasami::cli!` (no proc-macro)
or `#[derive(hasami::Args)]`.

| argh | hasami (`cli!` / derive) |
|------|--------------------------|
| `#[derive(FromArgs)]` | `hasami::cli! { struct .. }` or `#[derive(Args)]` |
| `/// doc` on the struct | same: the doc comment is the description |
| `#[argh(switch, short = 'v')]` on `bool` | `#[short = 'v']` / `#[hasami(short = 'v')]` on `bool` |
| `#[argh(option)]` on `Option<T>` | nothing needed: `Option<T>` is an optional value |
| `#[argh(option, default = "1")]` on `T` | `#[default = 1]` (typed expression, not a string) |
| `#[argh(option)]` on `Vec<T>` | nothing needed |
| `#[argh(positional)]` | `#[positional]` |
| `#[argh(positional, greedy)]` | `#[positional]` on `Vec<T>` (last positional) |
| `#[argh(subcommand)]` on `Option<Enum>` | `#[subcommand]` on `Option<Enum>` (or `Enum` for required) |
| `#[derive(FromArgs)] #[argh(subcommand)] enum` | `enum` inside `cli!` or `#[derive(Commands)]` |
| `#[argh(subcommand, name = "add")]` on the payload struct | `#[name = "add"]` on the **variant** |
| `#[argh(from_str_fn(f))]` | any `FromStr` type, or `Arg::value_with` in the builder |
| `argh::from_env()` | `Args::parse()` |
| `Args::from_args(&["cmd"], &["-v"])` | `Args::try_parse_args(["-v"])` |

Gains: `-abc` clusters, `--opt=val` and `-oval`, `--` handling, non-UTF-8
values, optional values, "did you mean", colour, env fallback. Build time
without `derive` is the same as argh's minus syn (about 0.7 s versus 3.2 s
on the reference machine).
