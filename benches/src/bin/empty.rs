//! Baseline: a `main` that only touches `std::env::args_os` and `println!`,
//! so the parts of `std` any CLI pays for are excluded from the diff.
#![forbid(unsafe_code)]

fn main() {
    let n = std::env::args_os().count();
    println!("{n}");
}
