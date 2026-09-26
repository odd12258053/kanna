# ADR-0008: The declarative IR and typed keys

Status: Accepted (2026-09-26)

## Context

SPEC.md §4.2 asks for a declarative builder with typed constraints, and
§4.3/§4.4 require that the `cli!` macro and `#[derive(Args)]` both lower to
the **same** intermediate representation, so that help, errors, completions
and schemas cannot diverge between front ends. The IR therefore has to be a
runtime value that can be built by hand, by `macro_rules!` and by a proc
macro alike, and it has to be introspectable by the generator crates.

## Decision

* The IR is `hasami::Command`: a plain, `Clone` value holding `ArgDef`s,
  `Subcommand`s, `Group`s and `requires` pairs. Parsing takes `&Command`
  and never mutates it. Every field has a public getter and no public
  setter other than the builder methods.
* **The argument definition is the key.** `Arg<T>` is a type-state builder
  (`Arg<bool>` flag → `.value::<U>()` → `Arg<Option<U>>` → `.required()`
  / `.default(v)` → `Arg<U>`, `.many()` → `Arg<Vec<U>>`, `.count()` →
  `Arg<usize>`). `Command::arg(&arg)` copies the untyped definition into
  the command; the caller keeps the `Arg<T>` and reads the result back with
  `Matches::get(&arg) -> T`. There is no separate `Key<T>` type and no
  stringly-typed `get::<T>("name")`.
* Values are parsed **at parse time** by a type-erased parser
  (`Arc<dyn Fn(&OsStr) -> Result<Box<dyn Any>, _>>`) stored in the
  definition, so every "invalid value" error is reported before the
  program runs, with the option name and the reason. `get` only downcasts.
* Identity is the argument **id** (a string; the long name by default).
  `Matches::get` looks the id up in a small `Vec` (linear scan: commands
  have a handful of arguments and a hash map would cost more code than it
  saves).
* `Matches::get` panics only when the `Arg` does not belong to the command
  that produced the matches. That is a programming error on par with
  indexing out of bounds; `try_get` returns `Option` for callers that
  prefer it. Parsing itself is total: no input can make it panic.
* Constraints are values on the command: `Group { members, required,
  exclusive }` and `requires(a, b)`. `Command::exclusive([...])` is sugar
  for an anonymous exclusive group. "Present" means given on the command
  line or through the environment; defaults never count.
* Subcommands are `Subcommand { name, about, aliases, hidden, build }`
  where `build` is either an `Arc<Command>` (eager) or an
  `Arc<dyn Fn() -> Command>` (lazy, SPEC.md §4.2 "deferred"). The parent's
  help needs only the name and summary, so lazy subcommands are built
  exclusively when selected or when a generator walks the tree.
* Global options (`Arg::global()`) are matched from any subcommand below
  the defining command and stored in the **defining command's** matches.
* Duplicate ids and a positional after a repeated positional are
  definition bugs; they are reported by `debug_assert!` in `Command::arg`.

## Consequences

* The `cli!` macro and the derive macro each generate (a) a function that
  builds `Arg`s and a `Command`, and (b) a `from_matches` that calls
  `Matches::get` with the same `Arg`s. Equality of the two IRs can be
  tested by comparing rendered help and schema output.
* `T: Clone` is required for values because `get` copies out of the
  matches; a `take`-style API can be added later without breaking anything.
* Each `.value::<U>()` monomorphises one small parser closure. This is the
  price of typed values without a proc macro and is measured by the size
  gate.
