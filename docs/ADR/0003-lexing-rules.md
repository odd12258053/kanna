# ADR-0003: Lexing rules for GNU/POSIX edge cases

Status: Accepted (2026-09-26)

## Context

Principle 1 of SPEC.md is full GNU/POSIX conformance. Several corners are
not specified by POSIX and libraries disagree. This ADR fixes what
`hasami-core` does so that higher layers, tests and documentation agree.

## Decision

| Input | Behaviour | Rationale |
|-------|-----------|-----------|
| `--` | Terminates option parsing; every later argument is a `Value`, including another `--`. `--` itself is never returned. | POSIX guideline 10. |
| `-` | A `Value`. | Universal convention for stdin/stdout. |
| `--opt=val`, `--opt=` | `Long("opt")` with an attached value (possibly empty). If the caller never asks for it, the next `next()` returns `Error::UnexpectedValue`. | Makes "flag given a value" an error for free, as in `getopt_long`. |
| `--opt val` | `value()` takes the next raw argument **whatever it looks like** (`--opt --weird` gives `--weird`; `--opt --` gives `--`). | GNU `getopt_long` behaviour; needed for values such as negative numbers (`-n -5`). |
| `-oval` | `Short('o')` with attached value `val`. `value()` and `optional_value()` return it. | POSIX guideline 5 / `getopt`. |
| `-o=val` | The leading `=` is **stripped**: value is `val`. `-o==val` gives `=val`. | Users expect `=` to be a separator symmetric with the long form (lexopt makes the same call). A value that genuinely starts with `=` can be passed as a separate argument. |
| `-abc` | `Short('a')`, `Short('b')`, `Short('c')`. If `b` takes a value, the value is `c`. | POSIX guideline 5. |
| `-a=b` where `a` is a flag | `Short('a')`, `Short('=')`, `Short('b')`. | Only the caller knows whether `a` takes a value; the lexer must not guess. |
| `---x`, `--=x` | `Long("-x")`, `Long("")` with value `x`. | Never reject syntactically; let the higher layer report "unknown option". |
| `--opt[=WHEN]` | `optional_value()` returns only attached text; a following argument is never consumed. | `getopt_long` `optional_argument` semantics; the only unambiguous rule. |
| `values()` (multi-value) | Yields the attached value, then following arguments until one starts with `-` (except the lone `-`) or `--`. `--opt=a b` and `-o=a b` yield only `a`; `-oa b` and `--opt a b` yield `a`, `b`. | Same rules as lexopt so users moving between the two see no surprise; the `=` form is the escape hatch to stop after one. |
| `values()` with nothing available | `Error::MissingValue` before any iteration. | A multi-value option with zero values is nearly always a user mistake. |
| `raw_args()` with a pending attached value | `Error::UnexpectedValue`. `try_raw_args()` returns `None` and keeps the value. | It is ambiguous whether `-xfoo`'s `foo` belongs to `-x` or to the raw tail. |
| Errors from `next()` | The parser stays usable; the offending state is cleared. | Lets callers collect several errors, and makes the state machine trivially total. |

Windows `/opt` syntax is out of scope (SPEC.md §6).

## Consequences

* The `tests/lexer.rs` suite is the executable form of this table.
* Higher layers must implement "does this option take a value?" themselves;
  the lexer never consumes a following argument unless asked.
