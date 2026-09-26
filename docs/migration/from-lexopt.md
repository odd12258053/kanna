# From lexopt

`kanna-core` is a superset of lexopt's API with the same shape, so most
programs port by changing the crate path.

| lexopt | kanna-core |
|--------|-------------|
| `lexopt::Parser::from_env()` | `kanna_core::Parser::from_env()` |
| `Parser::from_iter`, `Parser::from_args` | same |
| `use lexopt::prelude::*` | `use kanna_core::prelude::*` |
| `Arg::Short(c)`, `Arg::Long(s)`, `Arg::Value(v)` | same |
| `p.next()?`, `p.value()?`, `p.optional_value()`, `p.values()?`, `p.raw_args()?` | same; `optional_value()` returns `Result` (see below) |
| `p.bin_name()` | same, plus `bin_name_os()` |
| `arg.unexpected()` | same |
| `ValueExt::{parse, parse_with, string}` | same |
| `Error::{MissingValue, UnexpectedOption, UnexpectedArgument, UnexpectedValue, NonUnicodeValue, ParsingFailed, Custom}` | same variants (`#[non_exhaustive]`) |

Differences:

* `optional_value()` returns `Result<Option<OsString>, Error>`; it can
  fail only on targets that are neither Unix nor Windows.
* `values()` and `raw_args()` behave as lexopt's (ADR-0003); `RawArgs`
  additionally offers `peek`, `next_if`, `as_slice`.
* Errors from `next()` leave the parser usable so several can be collected.
* Non-Unicode bytes in a short cluster yield `Short('\u{FFFD}')` instead
  of an error; the raw bytes are recoverable with `value()`.
* Windows non-Unicode handling uses no `unsafe` (ADR-0004).

When you outgrow the loop, `kanna::Command::try_parse_with(&mut parser)`
lets the declarative layer take over an existing core parser.
