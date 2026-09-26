//! One generator per shell. Each produces a static script from the
//! [`Node`](crate::Node) tree and a dynamic wrapper script.

pub(crate) mod bash;
pub(crate) mod fish;
pub(crate) mod nushell;
pub(crate) mod powershell;
pub(crate) mod zsh;

/// Single-quote for POSIX shells.
pub(crate) fn sq(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}
