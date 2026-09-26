//! `hexdump`: show the bytes of files in hex and ASCII. `#[derive(Args)]`
//! with a custom `FromStr` type for byte sizes (`4k`, `0x100`, `2M`).
//!
//! ```text
//! cargo run -p kanna-example-hexdump -- -n 64 Cargo.lock
//! cargo run -p kanna-example-hexdump -- --skip 0x10 --width 8 --offset dec Cargo.toml
//! echo hello | cargo run -p kanna-example-hexdump
//! ```
#![forbid(unsafe_code)]

use std::fmt;
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use kanna::{Args, Cli};

/// Show the bytes of files in hex and ASCII
#[derive(Args)]
#[kanna(name = "hexdump", version = env!("CARGO_PKG_VERSION"))]
struct Opts {
    /// Stop after this many bytes (accepts 4k, 2M, 0x100)
    #[kanna(short = 'n', long = "length", value_name = "BYTES")]
    length: Option<Size>,
    /// Skip this many bytes from the start of each input
    #[kanna(short, value_name = "BYTES", default = Size(0))]
    skip: Size,
    /// Bytes per line
    #[kanna(short, default = 16)]
    width: usize,
    /// How to print the offset column
    #[kanna(value_name = "STYLE", default = String::from("hex"), possible = ["hex", "dec", "oct", "none"])]
    offset: String,
    /// Show every line instead of collapsing repeated ones into '*'
    #[kanna(short = 'v', long = "no-squeeze")]
    no_squeeze: bool,
    /// Files to dump (standard input when none)
    #[kanna(positional)]
    files: Vec<PathBuf>,
}

/// A byte count with an optional unit or a `0x` prefix.
#[derive(Clone, Copy, Debug)]
struct Size(u64);

impl FromStr for Size {
    type Err = String;
    fn from_str(s: &str) -> Result<Size, String> {
        if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            return u64::from_str_radix(hex, 16)
                .map(Size)
                .map_err(|e| format!("bad hexadecimal number: {e}"));
        }
        let (digits, unit) =
            s.split_at(s.trim_end_matches(|c: char| c.is_ascii_alphabetic()).len());
        let n: u64 = digits
            .parse()
            .map_err(|e| format!("bad number '{digits}': {e}"))?;
        let mult = match unit.to_ascii_lowercase().as_str() {
            "" | "b" => 1,
            "k" | "kb" => 1 << 10,
            "m" | "mb" => 1 << 20,
            "g" | "gb" => 1 << 30,
            _ => return Err(format!("unknown unit '{unit}' (use k, m or g)")),
        };
        n.checked_mul(mult)
            .map(Size)
            .ok_or_else(|| "size is too large".to_owned())
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn dump(o: &Opts, mut input: impl Read, out: &mut impl Write) -> io::Result<()> {
    let mut data = Vec::new();
    input.read_to_end(&mut data)?;
    let start = usize::try_from(o.skip.0)
        .unwrap_or(usize::MAX)
        .min(data.len());
    let end = match o.length {
        Some(Size(n)) => start
            .saturating_add(usize::try_from(n).unwrap_or(usize::MAX))
            .min(data.len()),
        None => data.len(),
    };
    let width = o.width.max(1);
    let mut previous: Option<&[u8]> = None;
    let mut squeezed = false;
    for (i, chunk) in data[start..end].chunks(width).enumerate() {
        if !o.no_squeeze && previous == Some(chunk) {
            if !squeezed {
                writeln!(out, "*")?;
                squeezed = true;
            }
            continue;
        }
        squeezed = false;
        previous = Some(chunk);
        let offset = start + i * width;
        match o.offset.as_str() {
            "hex" => write!(out, "{offset:08x}  ")?,
            "dec" => write!(out, "{offset:08}  ")?,
            "oct" => write!(out, "{offset:08o}  ")?,
            _ => {}
        }
        for (j, b) in chunk.iter().enumerate() {
            if j > 0 && j % 8 == 0 {
                write!(out, " ")?;
            }
            write!(out, "{b:02x} ")?;
        }
        // Pad short last lines so the ASCII column lines up.
        for j in chunk.len()..width {
            if j > 0 && j % 8 == 0 {
                write!(out, " ")?;
            }
            write!(out, "   ")?;
        }
        write!(out, " |")?;
        for &b in chunk {
            let c = if (0x20..0x7f).contains(&b) {
                b as char
            } else {
                '.'
            };
            write!(out, "{c}")?;
        }
        writeln!(out, "|")?;
    }
    match o.offset.as_str() {
        "hex" => writeln!(out, "{end:08x}"),
        "dec" => writeln!(out, "{end:08}"),
        "oct" => writeln!(out, "{end:08o}"),
        _ => Ok(()),
    }
}

fn main() -> ExitCode {
    let o = Opts::parse();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut status = ExitCode::SUCCESS;
    if o.files.is_empty() {
        if let Err(e) = dump(&o, io::stdin().lock(), &mut out) {
            eprintln!("hexdump: standard input: {e}");
            status = ExitCode::FAILURE;
        }
    }
    for path in &o.files {
        if o.files.len() > 1 {
            let _ = writeln!(out, "==> {} <==", path.display());
        }
        if let Err(e) = File::open(path).and_then(|f| dump(&o, f, &mut out)) {
            eprintln!("hexdump: {}: {e}", path.display());
            status = ExitCode::FAILURE;
        }
    }
    let _ = out.flush();
    status
}
