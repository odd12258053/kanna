//! `sift`: print lines that contain a pattern. The builder layer with the
//! `help`, `suggest` and `color` features.
//!
//! ```text
//! cargo run -p hasami-example-sift -- -rn "fn main" examples
//! cargo run -p hasami-example-sift -- -e hasami -e clap -il README.md
//! cargo run -p hasami-example-sift -- --recursve x .      # "did you mean"
//! ```
#![forbid(unsafe_code)]

use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hasami::{Arg, Command, Error, Matches};

struct Options {
    patterns: Vec<String>,
    ignore_case: bool,
    invert: bool,
    line_numbers: bool,
    count_only: bool,
    files_only: bool,
    recursive: bool,
    color: bool,
    paths: Vec<PathBuf>,
}

struct Args {
    pattern: Arg<Option<String>>,
    patterns: Arg<Vec<String>>,
    ignore_case: Arg<bool>,
    invert: Arg<bool>,
    line_numbers: Arg<bool>,
    count: Arg<bool>,
    files: Arg<bool>,
    recursive: Arg<bool>,
    color: Arg<String>,
    paths: Arg<Vec<PathBuf>>,
}

/// Keeping the `Arg`s together in a struct lets `command()` and
/// `Options::from` read the same typed handles.
fn args() -> Args {
    Args {
        pattern: Arg::positional::<String>("PATTERN").help("Text to look for"),
        patterns: Arg::new("regexp")
            .short('e')
            .value::<String>()
            .value_name("PATTERN")
            .many()
            .help("Pattern to look for; repeat to match any of several"),
        ignore_case: Arg::new("ignore-case")
            .short('i')
            .help("Ignore case distinctions"),
        invert: Arg::new("invert-match")
            .short('v')
            .help("Select non-matching lines"),
        line_numbers: Arg::new("line-number")
            .short('n')
            .help("Prefix each line with its line number"),
        count: Arg::new("count")
            .short('c')
            .help("Print only a count of matching lines per file"),
        files: Arg::new("files-with-matches")
            .short('l')
            .help("Print only the names of files with matches"),
        recursive: Arg::new("recursive")
            .short('r')
            .help("Search directories recursively"),
        color: Arg::new("color")
            .value::<String>()
            .value_name("WHEN")
            .possible(["auto", "always", "never"])
            .default_missing("always".to_owned())
            .default("auto".to_owned())
            .help("Highlight matches"),
        paths: Arg::positional::<PathBuf>("FILE")
            .many()
            .help("Files to search (standard input when none)"),
    }
}

fn command(a: &Args) -> Command {
    Command::new("sift")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Print lines that contain a pattern")
        .after_help(
            "The first positional argument is the pattern unless -e is given.\n\
             Exit status is 0 if a line was selected, 1 if none was, 2 on error.",
        )
        .arg(&a.patterns)
        .arg(&a.ignore_case)
        .arg(&a.invert)
        .arg(&a.line_numbers)
        .arg(&a.count)
        .arg(&a.files)
        .arg(&a.recursive)
        .arg(&a.color)
        .arg(&a.pattern)
        .arg(&a.paths)
        .exclusive([a.count.id(), a.files.id()])
}

impl Options {
    fn from(a: &Args, m: &Matches) -> Result<Options, Error> {
        let mut patterns = m.get(&a.patterns);
        let mut paths = m.get(&a.paths);
        // Without `-e`, the first positional is the pattern, the rest are files.
        match m.get(&a.pattern) {
            Some(first) if patterns.is_empty() => patterns.push(first),
            Some(first) => paths.insert(0, PathBuf::from(first)),
            None => {}
        }
        if patterns.is_empty() {
            return Err(Error::custom("no pattern given").with_tip("pass PATTERN or -e PATTERN"));
        }
        let ignore_case = m.get(&a.ignore_case);
        if ignore_case {
            for p in &mut patterns {
                *p = p.to_lowercase();
            }
        }
        let color = match m.get(&a.color).as_str() {
            "always" => true,
            "never" => false,
            _ => io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
        };
        Ok(Options {
            patterns,
            ignore_case,
            invert: m.get(&a.invert),
            line_numbers: m.get(&a.line_numbers),
            count_only: m.get(&a.count),
            files_only: m.get(&a.files),
            recursive: m.get(&a.recursive),
            color,
            paths,
        })
    }

    /// The byte range of the first match in `line`, if any.
    fn find(&self, line: &str) -> Option<(usize, usize)> {
        let haystack = if self.ignore_case {
            line.to_lowercase()
        } else {
            line.to_owned()
        };
        self.patterns
            .iter()
            .filter_map(|p| haystack.find(p.as_str()).map(|i| (i, i + p.len())))
            .min()
    }
}

struct Outcome {
    selected: bool,
    failed: bool,
}

fn search(
    o: &Options,
    name: &str,
    reader: impl BufRead,
    out: &mut impl Write,
    show_name: bool,
) -> io::Result<bool> {
    let mut selected = 0u64;
    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        let hit = o.find(&line);
        if hit.is_some() == o.invert {
            continue;
        }
        selected += 1;
        if o.count_only {
            continue;
        }
        if o.files_only {
            writeln!(out, "{name}")?;
            return Ok(true);
        }
        if show_name {
            write!(out, "{name}:")?;
        }
        if o.line_numbers {
            write!(out, "{}:", idx + 1)?;
        }
        match hit {
            Some((s, e)) if o.color && !o.invert => {
                // Highlighting uses the match on the case-folded text, whose
                // byte offsets agree with the original only for ASCII.
                if line.is_char_boundary(s) && line.is_char_boundary(e) {
                    writeln!(
                        out,
                        "{}\x1b[1;31m{}\x1b[0m{}",
                        &line[..s],
                        &line[s..e],
                        &line[e..]
                    )?;
                } else {
                    writeln!(out, "{line}")?;
                }
            }
            _ => writeln!(out, "{line}")?,
        }
    }
    if o.count_only {
        if show_name {
            writeln!(out, "{name}:{selected}")?;
        } else {
            writeln!(out, "{selected}")?;
        }
    }
    Ok(selected > 0)
}

fn walk(o: &Options, path: &Path, files: &mut Vec<PathBuf>, outcome: &mut Outcome) {
    if path.is_dir() {
        if !o.recursive {
            eprintln!("sift: {}: is a directory", path.display());
            outcome.failed = true;
            return;
        }
        match fs::read_dir(path) {
            Ok(entries) => {
                let mut entries: Vec<PathBuf> =
                    entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
                entries.sort();
                for entry in entries {
                    walk(o, &entry, files, outcome);
                }
            }
            Err(e) => {
                eprintln!("sift: {}: {e}", path.display());
                outcome.failed = true;
            }
        }
    } else {
        files.push(path.to_path_buf());
    }
}

fn run(o: &Options) -> Outcome {
    let mut outcome = Outcome {
        selected: false,
        failed: false,
    };
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    if o.paths.is_empty() {
        match search(o, "(standard input)", io::stdin().lock(), &mut out, false) {
            Ok(hit) => outcome.selected |= hit,
            Err(e) => {
                eprintln!("sift: standard input: {e}");
                outcome.failed = true;
            }
        }
        return outcome;
    }
    let mut files = Vec::new();
    for p in &o.paths {
        walk(o, p, &mut files, &mut outcome);
    }
    let show_name = files.len() > 1;
    for path in &files {
        let name = path.display().to_string();
        let result = fs::File::open(path)
            .and_then(|f| search(o, &name, io::BufReader::new(f), &mut out, show_name));
        match result {
            Ok(hit) => outcome.selected |= hit,
            Err(e) => {
                eprintln!("sift: {name}: {e}");
                outcome.failed = true;
            }
        }
    }
    let _ = out.flush();
    outcome
}

fn main() -> ExitCode {
    let a = args();
    let m = command(&a).parse();
    let o = Options::from(&a, &m).unwrap_or_else(|e| e.exit());
    let outcome = run(&o);
    if outcome.failed {
        ExitCode::from(2)
    } else if outcome.selected {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
