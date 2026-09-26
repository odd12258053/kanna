# Migration guides

hasami deliberately has **no** compatibility layer for other parsers
(SPEC.md §6). These guides map each library's concepts onto hasami's so a
port is mechanical.

* [From clap](from-clap.md)
* [From argh](from-argh.md)
* [From bpaf](from-bpaf.md)
* [From lexopt](from-lexopt.md)
* [From pico-args](from-pico-args.md)
* [From xflags](from-xflags.md)

Common to all: values are `OsString` until you ask for a type; `--help`
and `--version` are synthetic and can be disabled; every error is a
`hasami::Error` with `kind()`, `message()`, `tip()`, `usage()`; `parse()`
exits, `try_parse()` returns.
