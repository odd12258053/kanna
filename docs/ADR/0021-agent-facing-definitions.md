# ADR-0021: Agent-facing definitions: value help, `no_tool`, typed tool errors

Status: Accepted (2026-09-26)

## Context

A review of the `units` example (a CLI driven through the tool
definitions of ADR-0019) found four gaps between what a language model
needs and what a `Command` could express:

1. Value enums list only their spellings. An agent seeing
   `"enum": ["c", "f", "k"]` cannot tell Celsius from Kelvin, and there
   was no place in the definition for a description per value.
2. Every visible subcommand became a tool, including the driver's own
   commands (`tools`, `call`, `schema`). `hidden` was the only way to
   keep one out, and it also removes the command from `--help` and
   completion. `call` in particular takes a JSON string, so an agent
   could call it recursively.
3. `tool::to_argv` returned `Result<_, String>`. Applications wrapped
   the message in `Error::custom`, so a missing required argument and an
   invalid value both reached the agent as `"kind": "custom",
   "arg": null` and could only be told apart by reading the text.
4. Counters were emitted with `minimum: 0` but unsigned integers were
   not, so `-1` for a `usize` produced the parser's "invalid digit" text
   rather than a schema violation the agent could see up front.

## Decision

* **Per-value help lives in the definition.** `ValueEnum` gains
  `help(&self) -> Option<&'static str>` (default `None`). `value_enum!`
  and `#[derive(ValueEnum)]` fill it from the variant's `///` comment,
  which is how every other help text in kanna is written. The builder
  gets `Arg::possible_with_help([(name, text), ..])` for plain values,
  and `ArgDef::possible_value_help()` reads the pairs back.
  Consumers: `--help` renders `[possible values: c = Celsius, f, k]`
  (first line of each text; `-h` keeps the plain list), tool definitions
  append the same bracket to the property description, and the JSON
  document carries `possible_value_help`.
* **`no_tool` on subcommands.** `Subcommand::no_tool()`, `#[no_tool]`
  in `cli!` and `#[kanna(no_tool)]` in derive keep a subcommand (and its
  descendants) out of `tools()` while help, completion and documentation
  still show it. The flag is a property of the definition, like
  `hidden`, rather than a filter argument to `tools()`, so that all
  three front ends and the JSON document (`"no_tool"`) see the same
  thing.
* **`to_argv` returns `kanna::Error`.** Parser errors pass through with
  their `kind` and `arg`; a key that names no argument is
  `UnexpectedArgument` and a value of the wrong JSON shape is
  `InvalidValue`, both with the key as `arg`; malformed JSON and an
  unknown tool are `Custom`. The parser now also sets `arg` on a
  `MissingRequired` error when exactly one argument is missing, which is
  the usual case for a tool call.
* **`ValueType::Unsigned`.** Split from `Integer` at definition time;
  tool schemas add `minimum: 0` for it. The JSON document names it
  `"unsigned"`.

The document format is bumped to 3 for the new `value_type` value and
the two new keys. `to_argv`'s return type is a breaking change, made
while nothing depends on 0.1.0.

Not done: case-insensitive value matching (an agent writing `"C"` for
`c` is still rejected; the legend makes the expected spelling explicit)
and per-value help in `kanna-doc` output.

## Consequences

* A definition that documents its values once serves people (`--help`)
  and agents (tool schema) alike; the `units` example's unit properties
  read `c = Celsius, f = Fahrenheit, k = Kelvin`.
* `--help` output changes only for arguments that now have value help.
* `decl[help]` grows from 74.1 to 75.0 KiB (value help rendering, the
  unsigned split, the named missing argument), inside the 75 KiB budget
  of ADR-0018 with nothing to spare; the next addition to the builder
  will have to pay for itself.
* Consumers of the JSON document must accept format 3.
