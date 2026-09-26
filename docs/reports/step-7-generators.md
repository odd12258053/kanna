# Step 7 report: `hasami-complete`, `hasami-doc`, `hasami-schema`

Date: 2026-09-26.

## What was built

* `hasami-complete`: static scripts for bash, zsh, fish, PowerShell and
  nushell; dynamic completion (`Arg::complete_with`, `complete_from_env`,
  `generate_dynamic`) with a shell-independent environment protocol
  (ADR-0016). 9 tests, including a `bash -n` syntax check.
* `hasami-doc`: `manpage`, `manpage_with(ManOptions)`, `markdown`, `html`,
  recursive over subcommands. 4 tests.
* `hasami-schema`: `to_json`, `to_json_pretty`, `JSON_SCHEMA`,
  `FORMAT_VERSION`. 4 tests plus a doctest.
* `hasami/examples/extras.rs` exercising all three from one `Command`.

## Measurements

| Check | Result |
|-------|--------|
| `cargo test -p hasami-complete -p hasami-doc -p hasami-schema` | 19 tests + 3 doctests pass |
| clippy `-D warnings` (all features), `cargo doc -D warnings` | clean |
| Third-party dependencies of the three crates | 0 |
| Effect on `hasami` size/build gates | none (separate crates) |

## Design points that needed a decision

* **No re-export features on `hasami`** (ADR-0015): the generators need
  the IR, so they depend on `hasami`; a package cycle is not allowed even
  for optional dependencies. The spec's `complete`/`doc`/`schema` flags
  became separate crates you add to your own manifest.
* **`U+001F` as the word separator** in the dynamic protocol: NUL cannot
  be stored in environment variables; newline can appear inside quoted
  arguments; `U+001F` (unit separator) is what it was designed for.
* **Schema is hand-written JSON**, keeping the dependency count at zero;
  a JSON Schema constant documents the format instead of a serde model.
* **Hidden items are included in the schema but excluded from
  completion and docs**, matching the spec's "what the parser accepts"
  versus "what the user should see".
