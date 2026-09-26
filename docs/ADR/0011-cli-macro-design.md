# ADR-0011: `cli!` macro design (macro_rules, runtime specs)

Status: Accepted (2026-09-26)

## Context

SPEC.md §4.3 asks for a `macro_rules!`-only DSL that turns a struct-like
definition into a typed struct plus a `Command`, with doc comments as help
text, and with the same semantics as the future `#[derive(Args)]`.
`macro_rules!` cannot inspect types, concatenate identifiers, or transform
strings, so the design has to route around those limits.

## Decision

* **Field kind by token shape.** A tt-muncher tries, in order, the patterns
  `#[subcommand] f: Option<T>`, `#[subcommand] f: T`, `f: Option<T>`,
  `f: Vec<T>`, `f: bool`, `f: usize`, `f: T`. Fragments are only captured
  as `ty` after the shape is known, which is what makes the dispatch
  possible. `usize` gets its own arm so that `count` can be a runtime flag
  (`FieldSpec<usize>::usize_field` returns an `Arg<usize>` either way).
* **Settings are applied at runtime.** Each `#[a, b = x]` group is munched
  into method calls on a `FieldSpec<T>` (`s.short('n'); s.default(1);`).
  Ordering constraints of the type-state builder are handled inside the
  finalisers (`option()`, `vec()`, `plain()`, `flag()`), not by the macro.
  Kebab-casing of field and variant names is done at runtime too.
* **Attributes are classified by a pre-pass** (`__cli_attrs!`): each
  `#[...]` group becomes a doc line, a `derive(...)` to re-emit, or a
  settings list. This avoids the "local ambiguity" error that arises when
  `#[doc = ..]` and `#[$($tt)*]` repetitions are matched side by side.
* **No generated identifiers.** The typed `Arg`s are returned from a hidden
  `__spec()` as a tuple and destructured with the field names as bindings
  (`let (name, count, cmd) = Self::__spec();`), so neither `paste` nor
  index arithmetic is needed. Subcommand fields occupy a `()` slot.
* **Doc comments** reach the runtime as `#[doc = " text"]` literals; the
  single leading space is stripped per line at runtime. `ABOUT` is exposed
  as a raw `concat!` of the struct's doc lines so an enum can use the
  payload's summary when the variant has no doc comment.
* **Command name** defaults to `env!("CARGO_PKG_NAME")` evaluated at the
  expansion site, which is the user's crate. Subcommand entries override
  the name of the lazily built command (see `Subcommand::adopt`).
* `FieldSpec<T>` keeps everything type-independent in a non-generic
  `Common` struct so that only the few `T`-specific lines are
  monomorphised per value type. This cut the macro sample by 5.6 KiB.

## Consequences

* Types must be spelled literally as `Option<T>`, `Vec<T>`, `bool`,
  `usize`; `std::option::Option<T>` or a type alias is treated as a plain
  `T`. This is documented on the macro.
* Every value type needs `FromStr + Clone + Send + Sync + 'static` with a
  `Display` error, the same bound as `Arg::value`.
* The macro and the builder produce the same `Command`; the equivalence
  test in `kanna/tests/macro.rs` compares help, matches and errors between
  a hand-built and a macro-built definition. The derive macro (step 6)
  will be checked the same way.
* Cost: about 5.5 KiB on top of the builder for the sample CLI, paid only
  when the macro is used.
