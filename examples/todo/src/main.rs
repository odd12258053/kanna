//! `todo`: a todo list kept in a plain text file. The `cli!` macro with
//! subcommands, a global option with environment fallback, shell
//! completion generated from the same definition, and a REPL (`repl`)
//! from `kanna-prompt` that reads the same subcommands one line at a
//! time.
//!
//! ```text
//! cargo run -p kanna-example-todo -- add Buy milk
//! cargo run -p kanna-example-todo -- add --priority high Fix the roof
//! cargo run -p kanna-example-todo -- list
//! cargo run -p kanna-example-todo -- done 1
//! cargo run -p kanna-example-todo -- completions zsh
//! TODO_FILE=/tmp/other.txt cargo run -p kanna-example-todo -- list --all
//! cargo run -p kanna-example-todo -- repl
//! printf 'add Call mum\nlist\n' | cargo run -q -p kanna-example-todo -- repl
//! ```
//!
//! In the REPL, `--file` given on the command line stays in effect;
//! `help`, `exit` and end of input work as usual.
//!
//! File format, one task per line: `[ ] (A) text` for an open task with
//! priority A, `[x] text` for a finished one.
#![forbid(unsafe_code)]

use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

use kanna::{Cli, Error};
use kanna_complete::Shell;
use kanna_prompt::{Flow, Repl};

kanna::value_enum! {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Priority { High = "high", Normal = "normal", Low = "low" }
}

kanna::cli! {
    /// Keep a todo list in a text file
    #[name = "todo", version = env!("CARGO_PKG_VERSION")]
    #[after_help = "The list lives in ./todo.txt unless --file or TODO_FILE says otherwise."]
    struct Args {
        /// The list file [default: todo.txt]
        #[short, global, env = "TODO_FILE", value_name = "PATH"]
        file: Option<PathBuf>,
        #[subcommand] cmd: Cmd,
    }

    enum Cmd {
        /// Add a task
        Add(Add),
        /// Show open tasks
        #[alias = "ls"]
        List(List),
        /// Mark a task as finished
        Done(Select),
        /// Reopen a finished task
        Undo(Select),
        /// Delete a task
        #[name = "rm", alias = "remove"]
        Remove(Select),
        /// Delete every finished task
        Clear,
        /// Print a shell completion script
        Completions(Completions),
        /// Read subcommands from standard input, one per line
        #[no_tool]
        Repl,
    }

    struct Add {
        /// Importance
        #[short, value_enum, default = Priority::Normal]
        priority: Priority,
        /// Words of the task
        #[positional, required, value_name = "WORD"]
        text: Vec<String>,
    }

    struct List {
        /// Include finished tasks
        #[short] all: bool,
    }

    struct Select {
        /// Task number as shown by `list`
        #[positional] id: usize,
    }

    struct Completions {
        /// Which shell
        #[positional, possible = ["bash", "zsh", "fish", "powershell", "nushell"]]
        shell: Shell,
    }
}

impl Priority {
    fn letter(self) -> char {
        match self {
            Priority::High => 'A',
            Priority::Normal => 'B',
            Priority::Low => 'C',
        }
    }

    fn from_letter(c: u8) -> Option<Priority> {
        match c {
            b'A' => Some(Priority::High),
            b'B' => Some(Priority::Normal),
            b'C' => Some(Priority::Low),
            _ => None,
        }
    }
}

struct Task {
    done: bool,
    priority: Priority,
    text: String,
}

impl Task {
    fn parse(line: &str) -> Option<Task> {
        let (done, rest) = if let Some(rest) = line.strip_prefix("[x] ") {
            (true, rest)
        } else {
            (false, line.strip_prefix("[ ] ")?)
        };
        let (priority, text) = match rest.as_bytes() {
            [b'(', p, b')', b' ', ..] => (Priority::from_letter(*p)?, rest[4..].to_owned()),
            _ => (Priority::Normal, rest.to_owned()),
        };
        Some(Task {
            done,
            priority,
            text,
        })
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] ({}) {}",
            if self.done { 'x' } else { ' ' },
            self.priority.letter(),
            self.text
        )
    }
}

fn load(path: &PathBuf) -> Result<Vec<Task>, Error> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(Error::custom(format!(
                "cannot read {}: {e}",
                path.display()
            )));
        }
    };
    Ok(text.lines().filter_map(Task::parse).collect())
}

fn save(path: &PathBuf, tasks: &[Task]) -> Result<(), Error> {
    let mut text = String::new();
    for t in tasks {
        text.push_str(&t.to_string());
        text.push('\n');
    }
    fs::write(path, text)
        .map_err(|e| Error::custom(format!("cannot write {}: {e}", path.display())))
}

fn select(tasks: &mut [Task], id: usize) -> Result<&mut Task, Error> {
    id.checked_sub(1)
        .and_then(|i| tasks.get_mut(i))
        .ok_or_else(|| {
            Error::custom(format!("no task number {id}"))
                .with_tip("run 'todo list --all' to see the numbers")
        })
}

fn run(args: Args) -> Result<(), Error> {
    let file = args.file.unwrap_or_else(|| PathBuf::from("todo.txt"));
    let mut tasks = load(&file)?;
    match args.cmd {
        Cmd::Add(add) => {
            tasks.push(Task {
                done: false,
                priority: add.priority,
                text: add.text.join(" "),
            });
            save(&file, &tasks)?;
            println!("added task {}", tasks.len());
        }
        Cmd::List(list) => {
            let mut shown: Vec<(usize, &Task)> = tasks
                .iter()
                .enumerate()
                .filter(|(_, t)| list.all || !t.done)
                .collect();
            shown.sort_by_key(|(i, t)| (t.done, t.priority, *i));
            if shown.is_empty() {
                println!("nothing to do");
            }
            for (i, t) in shown {
                println!("{:>3}. {t}", i + 1);
            }
        }
        Cmd::Done(s) => {
            select(&mut tasks, s.id)?.done = true;
            save(&file, &tasks)?;
        }
        Cmd::Undo(s) => {
            select(&mut tasks, s.id)?.done = false;
            save(&file, &tasks)?;
        }
        Cmd::Remove(s) => {
            select(&mut tasks, s.id)?;
            let t = tasks.remove(s.id - 1);
            save(&file, &tasks)?;
            println!("removed: {}", t.text);
        }
        Cmd::Clear => {
            let before = tasks.len();
            tasks.retain(|t| !t.done);
            save(&file, &tasks)?;
            println!("removed {} finished task(s)", before - tasks.len());
        }
        Cmd::Completions(c) => {
            print!("{}", kanna_complete::generate(c.shell, &Args::command()));
        }
        Cmd::Repl => {
            // Each line is parsed into the same `Args`; the outer `--file`
            // carries over unless the line sets its own.
            let cmd = Args::command();
            Repl::new(&cmd)
                .prompt("todo> ")
                .run_typed::<Args, _>(|mut inner| {
                    if inner.file.is_none() {
                        inner.file = Some(file.clone());
                    }
                    match inner.cmd {
                        Cmd::Repl => Err(Error::custom("already in the repl")),
                        _ => run(inner).map(|()| Flow::Continue),
                    }
                })
                .map_err(Error::from)?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        e.exit();
    }
}
