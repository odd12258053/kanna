# ADR-0020: The project is named kanna

Status: Accepted (2026-09-26)

## Context

Preparing the first crates.io release showed that the name `hasami`
(鋏, scissors) was already registered by an unrelated crate (a clipboard
manager, 0.1.2). The five companion names (`hasami-core`, `-derive`,
`-complete`, `-doc`, `-schema`) were free, so only the main crate was
blocked. Options considered: publishing the main crate under a different
package name while keeping the library name (as RustCrypto did with
`md-5`/`md5`), asking the current owner for the name, or renaming the
whole family. Splitting package and library names was judged confusing
for people and for language models that generate `Cargo.toml` and `use`
lines together; asking the owner has no bounded timeline.

Candidate names were drawn from the same family of Japanese tool words
and checked, with all five companion names, against crates.io:
`wakachi` (分かち書き, splitting text into words), `kanna` (鉋, a plane
that shaves material away), `tsugite` (継手, joinery), `kogatana` (小刀),
`kiridashi` (切り出し), `kizami` (刻み), `kireme` (切れ目), `tsumu` (積む).

## Decision

The project, the main crate and the crate family are renamed to
**kanna** (鉋): a plane shaves off what is not needed, which is what the
library does to binary size and dependencies, and the name is five
letters like the other parsers people compare it with. Companion crates
are `kanna-core`, `kanna-derive`, `kanna-complete`, `kanna-doc`,
`kanna-schema`; the derive attribute is `#[kanna(...)]`; the environment
variables are `KANNA_ERROR_FORMAT` and `_KANNA_COMPLETE*`; the GitHub
repository is `odd12258053/kanna`. Every earlier document, including the
ADRs and step reports, was rewritten to the new name so that the
documentation set is consistent; this ADR records that the name before
2026-09-26 was hasami.

## Consequences

* Nothing had been published, so there is no compatibility shim and no
  deprecated crate.
* `wakachi` remains the natural name for a future tokenising component
  if one is ever split out of `kanna-core`.
* The local checkout directory may still be called `hasami`; only the
  repository, packages and identifiers changed.
