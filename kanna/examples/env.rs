//! Environment variable fallback, feature `env`. Each option reads its
//! value from the command line first, then from a variable, then from the
//! default; `Matches::source` tells which one won.
//!
//! Run: `cargo run --features env --example env -- --port 3000`
//!      `SERVE_PORT=8080 SERVE_HOST=0.0.0.0 cargo run --features env --example env`
//!      `SERVE_PORT=zzz cargo run --features env --example env`   (fails)
#![forbid(unsafe_code)]

use std::path::PathBuf;

use kanna::{Arg, Command, Source};

fn origin(source: Option<Source>) -> &'static str {
    match source {
        Some(Source::CommandLine) => "command line",
        Some(Source::Env) => "environment",
        Some(Source::Default) => "default",
        _ => "absent",
    }
}

fn main() {
    let host = Arg::new("host")
        .short('H')
        .value::<String>()
        .env("SERVE_HOST")
        .default("127.0.0.1".to_owned())
        .help("Address to bind");
    let port = Arg::new("port")
        .short('p')
        .value::<u16>()
        .env("SERVE_PORT")
        .default(8000)
        .help("TCP port");
    let root = Arg::new("root")
        .value::<PathBuf>()
        .env("SERVE_ROOT")
        .help("Directory to serve (current directory when unset)");
    // Environment values are validated exactly like command line values, so a
    // bad `SERVE_WORKERS` is reported as an invalid value, not silently ignored.
    let workers = Arg::new("workers")
        .short('j')
        .value::<usize>()
        .env("SERVE_WORKERS")
        .default(4)
        .help("Worker threads");

    let cmd = Command::new("serve")
        .version("0.1.0")
        .about("A static file server that reads its settings from the environment")
        .arg(&host)
        .arg(&port)
        .arg(&root)
        .arg(&workers);

    let m = cmd.parse();

    println!(
        "host    = {:<12} ({})",
        m.get(&host),
        origin(m.source(&host))
    );
    println!(
        "port    = {:<12} ({})",
        m.get(&port),
        origin(m.source(&port))
    );
    println!(
        "workers = {:<12} ({})",
        m.get(&workers),
        origin(m.source(&workers))
    );
    match m.get(&root) {
        Some(dir) => println!(
            "root    = {:<12} ({})",
            dir.display(),
            origin(m.source(&root))
        ),
        None => println!("root    = {:<12} ({})", ".", origin(m.source(&root))),
    }
}
