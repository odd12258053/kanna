# ADR-0004: Non-Unicode arguments without `unsafe`

Status: Accepted (2026-09-26)

## Context

Arguments may be arbitrary bytes on Unix and arbitrary UTF-16 (with unpaired
surrogates) on Windows. `OsStr` hides the encoding, and the only stable, safe
way to look inside is `OsStr::as_encoded_bytes` (UTF-8 on Unix, WTF-8 on
Windows). Building an `OsString` back from a *slice* of those bytes needs
`unsafe` (`from_encoded_bytes_unchecked`), which the workspace forbids.

## Decision

1. All scanning (`-`, `--`, `=`, short clusters) works on
   `as_encoded_bytes()`. Splitting happens only at ASCII bytes or on
   code-point boundaries, which is sound in both UTF-8 and WTF-8.
2. Long option names must be valid UTF-8 (`Arg::Long(&str)`). A non-Unicode
   name yields `Error::UnexpectedOption` with a lossy rendering. Names are
   identifiers chosen by the program author; there is no legitimate use for
   arbitrary bytes there.
3. Short option characters are decoded one at a time. An ill-formed unit
   yields `Short('\u{FFFD}')` and advances past the maximal ill-formed
   subsequence; a WTF-8 surrogate triple is treated as one unit so offsets
   stay on WTF-8 boundaries. The raw bytes are still recoverable through
   `value()`.
4. Rebuilding an `OsString` from a suffix:
   * if the suffix is valid UTF-8 (the overwhelmingly common case), `&str`
     conversion; no platform code involved;
   * Unix: `OsStringExt::from_vec` (safe);
   * Windows: decode WTF-8 to UTF-16 with a 30-line in-crate routine, then
     `OsStringExt::from_wide` (safe). The routine is compiled and unit-tested
     on every platform under `cfg(test)`;
   * other targets: `Error::NonUnicodeValue`. This mirrors lexopt and affects
     only non-Unicode attached values on exotic targets.

## Consequences

* Zero `unsafe`, zero dependencies, and the lexer never panics on any byte
  sequence (verified by the fuzz harness, ADR-0006).
* `Arg::Short` can carry `U+FFFD`; higher layers that render "unknown
  option -\u{FFFD}" should say so plainly rather than pretend it is a real
  character. Values remain exact bytes.
* Windows behaviour is tested only through the WTF-8 routine's unit tests
  until a Windows CI job exists.
