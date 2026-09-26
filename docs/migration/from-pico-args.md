# From pico-args

pico-args pulls options out of the argument list by name, in any order;
kanna-core walks the arguments in order, and `kanna::Command` declares
them. Either replacement removes pico-args' known gaps (`-abc` clusters,
`--opt=` handling, help generation).

| pico-args | kanna-core | kanna builder |
|-----------|-------------|----------------|
| `Arguments::from_env()` | `Parser::from_env()` | `Command::new(..)` then `.parse()` |
| `args.contains(["-v", "--verbose"])` | `Short('v') \| Long("verbose") => ..` | `Arg::new("verbose").short('v')`; `m.get(&verbose)` |
| `args.opt_value_from_str(["-n", "--number"])?` | `Short('n') \| Long("number") => p.value()?.parse()?` | `Arg::new("number").short('n').value::<u32>()` |
| `args.value_from_str("--x")?` (required) | same + a `None` check | `.value::<T>().required()` |
| `args.opt_value_from_fn("--x", f)?` | `p.value()?.parse_with(f)?` | `.value_with(f)` |
| `args.opt_value_from_os_str(..)` | `p.value()?` (already `OsString`) | `.value_os()` / `.value_os_with(f)` |
| `args.values_from_str("--x")?` | collect in the loop | `.many()` |
| `args.free_from_str()?` | `Value(v) => ..` | `Arg::positional::<T>("X").required()` |
| `args.subcommand()?` | `Value(v)` then `Command::try_parse_with` | `.subcommand(..)` |
| `args.finish()` (leftovers) | the loop's `_ => Err(arg.unexpected())` | automatic `unexpected argument` error |
| hand-written `--help` text | hand-written | generated |
