---
name: kanna
description: Build or change a command line interface with the kanna crates (kanna-core lexer, kanna builder, cli! macro, derive, and the completion/doc/schema generators). Use when writing Rust CLI argument parsing in this repository or any project that depends on kanna.
---

# Writing a CLI with kanna

1. **Read `docs/ai/kanna-reference.md` first.** It is the complete list of
   public names and settings. Do not guess from clap: kanna has no
   `#[arg]`, `#[command]`, `ArgAction`, `value_parser!`, `get_one`,
   `.index()`, `.num_args()`. The reference ends with a clap → kanna table.

2. **Pick the layer** (reference §1): `kanna-core` for a lexer loop with
   hand-written help; the builder for typed options and subcommands;
   `cli!` for a struct without a proc-macro; `#[derive(Args)]` when the
   `syn` build cost is acceptable. Enable only the features you use
   (`help` is on by default; `suggest`, `color`, `env`, `json`, `derive`).

3. **Write the definition, then prove it.** Every CLI gets, at minimum:
   * `example = "..."` lines (or `Command::example`) for the main uses;
   * a test that calls `Args::command().validate().unwrap()` and
     `.check_examples().unwrap()`;
   * `try_parse_args` tests for one success and one error per feature
     used, checking `e.kind()`.
   Parsing in debug builds runs `validate()` automatically and panics with
   every problem, so run the tests before reasoning about help output.

4. **Behaviour to remember** (reference §5): repeated single-value
   options and flags are errors unless `last_wins`; optional values must
   be attached (`--color=never`); `--n -1` takes `-1` as the value; ids are
   field names and relations (`requires = "field"`) refer to ids; the
   field order is the positional order; a required positional cannot be
   combined with subcommands.

5. **For agent-facing tools** use `kanna_schema::tool::tools(&cmd)` for
   Claude/MCP tool definitions and `tool::to_argv` to turn a tool call
   into a command line; set `KANNA_ERROR_FORMAT=json` (feature `json`)
   to get structured errors on stderr.

6. **Keep the budgets.** In this repository, run
   `benches/size.sh --check` and `benches/build-time.sh --check` when
   touching `kanna` or `kanna-core`; record decisions as ADRs in
   `docs/ADR/` and numbers in `docs/reports/`.

Worked, runnable examples: `kanna/examples/*.rs` (one feature each) and
`examples/{wc,sift,todo,hexdump}` (complete tools, one per layer).
