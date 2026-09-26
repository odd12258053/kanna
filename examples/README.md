# Example applications

Complete, working command line tools, one per hasami layer. Each is a
workspace member with its own `Cargo.toml`, so it shows exactly which
crate and which features a real program needs. None is published.

| Directory | Binary | Layer | Features | What it does |
|-----------|--------|-------|----------|--------------|
| `wc/` | `wc` | `hasami-core` | none | Counts lines, words, bytes and characters. Help, version and usage text are hand-written; the binary depends on `std` alone. |
| `sift/` | `sift` | builder | `help`, `suggest`, `color` | A substring grep: `-e` patterns, `-i`, `-v`, `-n`, `-c`, `-l`, `-r`, `--color[=WHEN]`, grep-compatible exit status. |
| `todo/` | `todo` | `cli!` macro | `help`, `suggest`, `env` | A todo list in a text file: `add`, `list`, `done`, `undo`, `rm`, `clear`, plus `completions` generated with `hasami-complete`. `--file` falls back to `TODO_FILE`. |
| `hexdump/` | `hexdump` | `#[derive(Args)]` | `help`, `derive` | Hex and ASCII dump with a custom `Size` type (`4k`, `0x100`), `--skip`, `--length`, `--width`, `--offset`, squeezing of repeated lines. |

Run one with `cargo run -p hasami-example-<name> -- <args>`:

```text
cargo run -p hasami-example-wc -- -lw Cargo.toml README.md
cargo run -p hasami-example-sift -- -rn "fn main" examples
cargo run -p hasami-example-todo -- add --priority high Fix the roof
cargo run -p hasami-example-hexdump -- -n 64 Cargo.lock
```

Shorter, single-file demonstrations of individual features live in
`hasami/examples/` and run with `cargo run -p hasami --example <name>`.
