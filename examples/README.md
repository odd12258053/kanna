# Example applications

Complete, working command line tools, one per kanna layer, plus one
that an AI agent can drive through `kanna-schema`. Each is a
workspace member with its own `Cargo.toml`, so it shows exactly which
crate and which features a real program needs. None is published.

| Directory | Binary | Layer | Features | What it does |
|-----------|--------|-------|----------|--------------|
| `wc/` | `wc` | `kanna-core` | none | Counts lines, words, bytes and characters. Help, version and usage text are hand-written; the binary depends on `std` alone. |
| `sift/` | `sift` | builder | `help`, `suggest`, `color` | A substring grep: `-e` patterns, `-i`, `-v`, `-n`, `-c`, `-l`, `-r`, `--color[=WHEN]`, grep-compatible exit status. |
| `todo/` | `todo` | `cli!` macro | `help`, `suggest`, `env` | A todo list in a text file: `add`, `list`, `done`, `undo`, `rm`, `clear`, plus `completions` generated with `kanna-complete`. `--file` falls back to `TODO_FILE`. |
| `hexdump/` | `hexdump` | `#[derive(Args)]` | `help`, `derive` | Hex and ASCII dump with a custom `Size` type (`4k`, `0x100`), `--skip`, `--length`, `--width`, `--offset`, squeezing of repeated lines. |
| `units/` | `units` | `cli!` + `kanna-schema` | `help`, `json` | A unit converter an AI agent can drive: `length`, `mass`, `temp`, plus `tools` (Claude / MCP tool definitions), `call` (runs a tool from its JSON input through `to_argv`) and `schema` (the JSON description). `KANNA_ERROR_FORMAT=json` gives structured errors. |

Run one with `cargo run -p kanna-example-<name> -- <args>`:

```text
cargo run -p kanna-example-wc -- -lw Cargo.toml README.md
cargo run -p kanna-example-sift -- -rn "fn main" examples
cargo run -p kanna-example-todo -- add --priority high Fix the roof
cargo run -p kanna-example-hexdump -- -n 64 Cargo.lock
cargo run -p kanna-example-units -- call units_temp '{"value": 100, "from": "c", "to": "f"}'
```

Shorter, single-file demonstrations of individual features live in
`kanna/examples/` and run with `cargo run -p kanna --example <name>`.
