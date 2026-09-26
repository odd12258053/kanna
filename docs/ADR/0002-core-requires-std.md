# ADR-0002: The core requires `std`; `alloc`-only is deferred

Status: Accepted (2026-09-26)

## Context

SPEC.md lists a `std` default feature "to leave room for an `alloc`-only
mode". The spec also mandates that the internal representation is
`OsString`/`OsStr`, which exist only in `std` (they encapsulate the
platform's native string encoding).

## Decision

* `kanna-core` declares a `std` feature that is on by default, but the
  crate does not compile without it today. The feature exists so that turning
  the crate `no_std + alloc` later is an additive change rather than a
  breaking one.
* An `alloc`-only mode, if ever added, would have to parse `&[u8]` (or
  `Vec<u8>`) arguments and would lose the platform-correct Windows handling
  that `OsStr` gives us. That trade-off is not worth making before there is a
  concrete user for it.
* `cargo test --no-default-features` must keep passing; it currently
  exercises the same code as the default build.

## Consequences

* No `#![no_std]` attribute in the core yet, and no code paths that would
  need it.
* The `std` feature of `kanna` forwards to `kanna-core/std` so that a
  future `alloc`-only core is reachable from the facade.
