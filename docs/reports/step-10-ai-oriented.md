# Step 10 report: features for AI-assisted CLI development

Date: 2026-09-26. Machine and toolchain as in the step 4/5 report
(24-core x86_64 Linux, rustc 1.98.1). Decisions in ADR-0019. Five of the
twelve proposals were chosen: the reference and skill, `cli!` setting
diagnostics, examples, tool definitions, and structured errors.

## What was built

| Item | Where | Notes |
|------|-------|-------|
| One-page reference | `docs/ai/hasami-reference.md` | every public name and setting, the parsing rules, a testing recipe, clap → hasami table |
| Claude Code skill | `.claude/skills/hasami/SKILL.md` | workflow: read the reference, pick a layer, prove the definition |
| Unknown-setting diagnostics | `hasami/src/macros.rs` | fallback arms with `compile_error!` naming the setting and listing the valid ones, for field, command and variant settings; `hasami/tests/ui/` (trybuild, feature `derive`) pins three messages, verified on stable and 1.85 |
| Examples | `Command::example`, `example = ".."` (cli!/derive), `Command::check_examples`, `get_examples` | `Examples:` section in help after the options; `.SH EXAMPLES` / `## Examples` / `<h>Examples` in the doc crate; `examples` in the JSON schema; a shell-like word splitter (`shell_words`) handles quotes and `# comments` |
| Value types | `ValueType`, `ArgDef::value_type` | recorded from the Rust type at definition time; `value_type` in the JSON schema |
| Tool definitions | `hasami_schema::tool` (`Tool`, `tools`, `to_json`, `Tool::to_json` / `to_mcp_json`, `to_argv`) | one tool per runnable command; JSON Schema inputs; a 150-line JSON reader keeps the crate dependency-free; `to_argv` validates by parsing |
| Structured errors | feature `json`: `Error::to_json`, `Error::arg` / `with_arg`, `ErrorKind::name`, `HASAMI_ERROR_FORMAT=json` | the parser names the argument for value, missing-value, repeat, requires and conflict errors |
| Example program | `hasami/examples/extras.rs` `tools` subcommand | prints definitions or converts a call |

Tests added: 4 in `tests/extended.rs`, 1 each in `tests/macro.rs` and
`tests/derive.rs`, 3 compile-fail cases, 5 in `hasami-schema::tool`.
Workspace total: 220 tests, green with default, no and all features;
fmt, clippy (both feature modes, `-D warnings`), rustdoc `-D warnings`.

## Measurements

| Variant | Before (step 9) | After | Budget |
|---------|----------------:|------:|-------:|
| core | 13.5 KiB | 13.5 KiB | ≤ 20 |
| decl[no features] | 57.1 KiB | 58.6 KiB | |
| decl[help] | 71.8 KiB | 74.1 KiB | ≤ 75 |
| decl[full] (now includes `json`) | 87.6 KiB | 93.2 KiB | |
| macro[help] | 79.3 KiB | 81.4 KiB | |
| decl[full] / clap | 0.44 | 0.47 | < 0.5 |

| Variant | dev | release | Budget |
|---------|----:|--------:|-------:|
| core | 0.82 s | 1.10 s | ≤ 2 s |
| decl[help] | 0.88 s | 1.27 s | |
| decl[full] | 0.90 s | 1.39 s | ≤ 4 s |
| macro[full] | 0.94 s | 1.42 s | |

Both gates pass, with 0.9 KiB left under the `help` budget and 0.03
under the clap ratio. The next feature that touches the default build
will need either a size offset or a budget decision.

## Design points that needed a decision

* **`Error` grew past clippy's 128-byte threshold** with the `arg` field.
  The optional fields (`tip`, `usage`, `arg`, `source`) moved into one
  `Option<Box<Details>>`; the public accessors are unchanged and every
  `Result<T, Error>` shrank.
* **Examples are parsed from the command they belong to.** A subcommand's
  example may start with `app sub` or `sub`; the words up to and
  including the command's own name are dropped. `check_examples` is a
  separate call, because parsing already runs `validate` and the two
  must not recurse.
* **Tool property names are the argument ids**, positionals included
  (`FILE`), so `to_argv` needs no separate mapping and the schema's
  `required` list is the parser's.
* **Long names preferred in `to_argv`** (`--verbose --verbose` rather
  than `-v -v`) so the generated line reads like the help text; a value
  is always attached (`--count=2`) so it can start with `-`.
* **`json` is a feature, off by default**, because the JSON writer and
  the environment check cost 5.6 KiB and only agent-driven programs need
  it; it is part of `full`.
* **Reference over README.** The README stays a tour; only the reference
  promises to list everything, so a model has one file to read.

## Not done / follow-ups

* The remaining proposals: `Command::lint()` warnings, a testing helper
  module, a self-describing `--help-json` flag, an LLM-oriented help
  renderer, `Matches::explain()`, a written stability policy for exit
  codes and messages.
* The size headroom is nearly used up; ADR-0012's budget will need
  another look before the next default-build feature.
