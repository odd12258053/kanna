# From xflags

xflags describes the interface with a small grammar inside `xflags::xflags!`
and generates a struct plus a hand-rolled parser. `kanna::cli!` plays the
same role with a syntax closer to Rust and lowers to a runtime `Command`,
which is what gives it help, suggestions, completions and schemas.

| xflags | kanna `cli!` |
|--------|---------------|
| `xflags::xflags! { cmd app { .. } }` | `kanna::cli! { struct App { .. } }` |
| `optional -v, --verbose` | `#[short] verbose: bool` |
| `optional -n, --number n: u32` | `#[short] number: Option<u32>` |
| `required --name s: String` | `name: String` |
| `repeated -I, --include s: String` | `#[short = 'I'] include: Vec<String>` |
| `optional path: PathBuf` (positional) | `#[positional] path: Option<PathBuf>` |
| `required path: PathBuf` | `#[positional] path: PathBuf` |
| `repeated paths: PathBuf` | `#[positional] paths: Vec<PathBuf>` |
| `cmd sub { .. }` (nested) | `#[subcommand] cmd: Option<Sub>` + `enum Sub { Name(Payload) }` |
| `default cmd` | not supported; use `Option<Sub>` and handle `None` |
| `/// docs` | `/// docs` |
| `App::from_env_or_exit()` | `App::parse()` |
| `App::from_env()` → `Result` | `App::try_parse()` |
| `App::from_vec(args)` | `App::try_parse_args(args)` |
| `src = "path"` (dump generated code) | not offered; `cargo expand` works on `cli!` |

Behavioural upgrades: `-abc`, `--opt=val`, `-oval`, optional values,
non-UTF-8 values, "did you mean", colour, environment fallback, and the
generated `Command` is introspectable at runtime.
