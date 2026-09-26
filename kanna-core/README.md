# kanna-core

A dependency-free, panic-free, non-Unicode-safe GNU/POSIX command line
**lexer**. This is the bottom layer of [kanna](../README.md); it is also a
complete, tiny argument parser on its own in the style of `lexopt`.

* one source file, `#![forbid(unsafe_code)]`, no dependencies, no macros
* `OsString` everywhere: arguments that are not valid Unicode pass through
* handles `--opt=val`, `--opt val`, `-oval`, `-o val`, `-o=val`, `-abc`,
  `--`, `-`, optional values (`--color[=WHEN]`), multi-values, raw tails
* every error leaves the parser usable, so several errors can be collected
* MSRV 1.85

```rust
use kanna_core::prelude::*;

fn main() -> Result<(), kanna_core::Error> {
    let mut number = 1u32;
    let mut shout = false;
    let mut thing: Option<String> = None;

    let mut parser = Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('n') | Long("number") => number = parser.value()?.parse()?,
            Long("shout") => shout = true,
            Value(v) => thing = Some(v.string()?),
            _ => return Err(arg.unexpected()),
        }
    }
    // ...
    Ok(())
}
```

The exact lexing rules are recorded in
[ADR-0003](../docs/ADR/0003-lexing-rules.md); non-Unicode handling in
[ADR-0004](../docs/ADR/0004-non-unicode-handling.md).

## Testing

```sh
cargo test -p kanna-core                      # unit + behavioural + pseudo-fuzz
cd kanna-core && cargo +nightly fuzz run lexer   # libFuzzer (needs nightly)
```
