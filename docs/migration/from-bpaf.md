# From bpaf

bpaf builds a parser value by combinators; kanna builds a `Command` value
and reads typed results back through the `Arg`s. Constraints that bpaf
expresses with combinators (`.or_else`, `.guard`) are `Group`s and
`requires` here; arbitrary validation is a `value_with` parser.

| bpaf | kanna |
|------|--------|
| `short('v').long("verbose").switch()` | `Arg::new("verbose").short('v')` |
| `short('n').long("number").argument::<u32>("N")` | `Arg::new("number").short('n').value::<u32>().value_name("N")` |
| `.optional()` | default for `.value()` (`Arg<Option<T>>`) |
| `.fallback(1)` | `.default(1)` |
| `.many()` / `.some("msg")` | `.many()` / `.many().required()` |
| `.req_flag(())` | flag + `Group::required()` |
| `positional::<T>("FILE")` | `Arg::positional::<T>("FILE").required()` |
| `a.or_else(b)` / `construct!([a, b])` | `Command::exclusive([a.id(), b.id()])` or a `Group` |
| `.guard(f, "msg")` | `.value_with(|s| ..)` returning `Err("msg")` |
| `.complete(f)` | `.complete_with(f)` + `kanna-complete` |
| `command("name", parser)` | `Command::subcommand(Command::new("name")..)` |
| `.to_options().descr("..").version("..")` | `Command::new("app").about("..").version("..")` |
| `.run()` | `cmd.parse()` |
| `.run_inner(args)` | `cmd.try_parse_args(args)` |
| `#[derive(Bpaf)]` | `kanna::cli!` or `#[derive(Args)]` |

Things bpaf does that kanna does not: parsers as first-class values you
can combine arbitrarily (`construct!` with `Option`s of structs), and
adjacent-argument grouping. Things kanna adds: lazy subcommands, a
zero-dependency default, and generators for completion/docs/schema that
see the whole tree.
