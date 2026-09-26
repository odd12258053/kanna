# ADR-0015: Generator crates depend on `kanna`; `kanna` does not re-export them

Status: Accepted (2026-09-26). Amends ADR-0001.

## Context

SPEC.md §3 lists `complete`, `doc` and `schema` as feature flags on the
`kanna` crate that "enable the sub-crates". The generators need to walk
the `Command` IR, which lives in `kanna`. A crate cannot both depend on
`kanna` and be an optional dependency of it: Cargo rejects the package
cycle even when the edge is optional.

## Decision

* `kanna-complete`, `kanna-doc` and `kanna-schema` depend on `kanna`
  and are used directly (`kanna_schema::to_json(&cmd)`); the `kanna`
  crate has no `complete`/`doc`/`schema` features. `full` means
  `help + suggest + color + env`.
* The generators use only the public introspection API of `Command`,
  `ArgDef`, `Subcommand` and `Group`. Anything they need that is missing
  is added as a public getter, never as a private hook.
* Build-time and size budgets are unaffected: nothing in the default
  graph changes.

## Consequences

* A CLI that wants a `completions` subcommand adds `kanna-complete` to
  its own `[dependencies]`.
* The alternative, moving the IR into a fourth crate that both sides
  depend on, was rejected: it would split the documentation and add a
  crate boundary to the hot path for no user-visible gain.
