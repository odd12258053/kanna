# ADR-0005: Core API shape: imperative, borrowing `Arg`, sealed `ValueExt`

Status: Accepted (2026-09-26)

## Context

SPEC.md §4.1 asks for a lexopt-style imperative API: options arrive in order,
the caller matches on them and pulls values on demand. Several details are
worth fixing explicitly because they shape every layer above.

## Decision

* `Parser::next(&mut self) -> Result<Option<Arg<'_>>, Error>`. It is not an
  `Iterator` because `Arg::Long` borrows the parser and because the natural
  loop is `while let Some(arg) = p.next()?`.
* `Arg::Long(&str)` borrows a buffer inside the parser rather than allocating
  a `String` per option. This makes `Long("number") => p.value()?` pattern
  matching work without allocation; the cost is that a bound name cannot be
  held across another parser call, which is the same restriction lexopt has
  and is caught at compile time.
* `Arg::Short(char)`, not `u8`: option letters may be non-ASCII.
* `Arg::Value(OsString)` owns its data; no lifetime on positional values.
* Arguments are collected into a `Vec` up front (`vec::IntoIter`) instead of
  boxing a `dyn Iterator`. This gives cheap one-element lookahead for
  `values()` and `RawArgs::peek`, and command lines are small.
* `Parser::from_iter` takes the binary name as the first item (like
  `std::env::args_os`); `Parser::from_args` takes none. `bin_name()` returns
  `Option<&str>` and `bin_name_os()` the raw form.
* `Error` is `#[non_exhaustive]` so variants can be added before 1.0 without a
  breaking change. It carries `Box<dyn Error + Send + Sync>` for parse
  failures and custom errors, and implements `From<&str>`/`From<String>` so
  application code can `?` plain messages.
* `ValueExt` (`parse`, `parse_with`, `string`) is implemented for `OsString`
  only and is **sealed**: it is a convenience surface, not an extension point,
  so adding methods later is not a breaking change.
* `RawArgs` exposes `peek`, `next_if` and `as_slice`, which is what the
  declarative layer needs to implement subcommands and `--` pass-through
  without reaching into the parser.

## Consequences

* The declarative layer (`hasami`) is built entirely on this public API; it
  gets no private hooks. If it turns out to need one, that is a signal the
  core API is missing something for everyone.
* Error messages in the core say **what** went wrong and **which option**;
  "how to fix it" (suggestions, usage) is the job of the layer that knows the
  full command definition.
