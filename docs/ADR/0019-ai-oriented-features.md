# ADR-0019: Features for CLIs written by, and used by, AI agents

Status: Accepted (2026-09-26)

## Context

Two situations were studied: a language model writing a CLI with hasami,
and an agent calling a finished CLI as a tool. The first fails on
hallucinated settings and missing tests; the second fails on parsing
`--help` text and free-form error messages. Twelve candidates were
proposed; five were chosen for the first batch.

## Decision

1. **One-page reference and a Claude Code skill.**
   `docs/ai/hasami-reference.md` lists every public name and setting with
   the parsing rules and a clap → hasami table; `.claude/skills/hasami/SKILL.md`
   is the workflow (read the reference, pick a layer, prove the
   definition with `validate` and `check_examples`). The reference is the
   only document that promises completeness; the README stays a tour.

2. **`cli!` rejects unknown settings with a list of valid ones.** A
   fallback arm in each settings muncher emits `compile_error!` with the
   setting name and the full list, matching what the derive already did.
   `hasami/tests/ui/` pins the messages with `trybuild` (feature
   `derive`, so the test runs in the all-features CI job).

3. **Examples in the definition.** `Command::example("app add cake")`
   (setting `example = ".."`, repeatable) renders an `Examples:` section
   after the options and before `after_help`, appears in man/Markdown/HTML
   output and the JSON schema, and is verified by
   `Command::check_examples()`, which parses each example from the command
   it is attached to (dropping the words up to the command's own name and
   any `# comment`) and returns the failures. `check_examples` is not run
   automatically: it parses, and the automatic `validate` runs inside
   parsing.

4. **Tool definitions from a `Command`** (`hasami_schema::tool`).
   `tools(&cmd)` yields one `Tool` per runnable command (the root unless
   `subcommand_required`, and every subcommand) with a JSON Schema input
   built from the arguments: ids as property names, flags → boolean,
   counters → integer, values typed by the new `ValueType` recorded at
   definition time from the Rust type (`Integer`, `Float`, `Boolean`,
   `String`, `Path`, `Other`), `possible` → `enum`, `many` → array,
   `required` → required, hidden arguments omitted, parents' globals
   included. `Tool::to_json` (Claude: `input_schema`) and `to_mcp_json`
   (`inputSchema`). `to_argv(&cmd, name, json)` converts a call back into
   `Vec<OsString>` (`--long=value`, flags, repeated counters, positionals
   after `--` when needed) and checks it by parsing. The crate keeps zero
   dependencies with a 150-line JSON reader.

5. **Structured errors** (feature `json`, part of `full`).
   `Error::to_json()` gives `{"kind","exit_code","arg","message","tip","usage"}`
   with `ErrorKind::name()` as the stable kind string; `Error::arg()` /
   `with_arg` name the argument concerned (the parser sets it for
   value, missing-value, repeat, requires and conflict errors);
   `HASAMI_ERROR_FORMAT=json` switches `print`/`exit` to one JSON line on
   stderr for usage errors (help and version stay text on stdout). The
   feature is off by default because the JSON writer and the env check
   cost about 5 KiB.

## Consequences

* Size: `decl[help]` 71.8 → 74.1 KiB (examples section, `Error::arg`,
  value type); `decl[full]` 87.6 → 93.2 KiB with `json`; `full / clap`
  0.44 → 0.47, still under the 0.5 gate but with little room left. The
  optional `Error` fields were boxed on the way (`Result<T, Error>` is
  now 48 bytes instead of 145), which kept clippy's `result_large_err`
  quiet and cost nothing measurable.
* The JSON schema document gains `examples` and `value_type`; the format
  number stays 2 (unreleased).
* Proposals not taken up yet: `Command::lint()` warnings, a testing
  helper module, a self-describing `--help-json` flag, an LLM-oriented
  help renderer, `Matches::explain()`, a documented stability policy for
  exit codes and messages.
