//! Tool definitions for AI agents.
//!
//! An agent (Claude, an MCP client, ...) calls tools by name with a JSON
//! object as input. [`tools`] turns a [`Command`] into one such definition
//! per runnable command (the root when it can run without a subcommand,
//! and every subcommand likewise), each with a JSON Schema for its input
//! derived from the argument definitions. [`to_argv`] goes the other way:
//! it turns an agent's JSON input back into the argument vector the
//! command expects, and checks it by parsing.
//!
//! ```
//! use hasami::{Arg, Command};
//! use hasami_schema::tool;
//!
//! let count = Arg::new("count").short('c').value::<u32>().default(1).help("Repeat");
//! let loud = Arg::new("loud").help("Shout");
//! let file = Arg::positional::<String>("FILE").required().help("Input");
//! let cmd = Command::new("greet").about("Greet someone").arg(&count).arg(&loud).arg(&file);
//!
//! let tools = tool::tools(&cmd);
//! assert_eq!(tools[0].name, "greet");
//! assert!(tools[0].input_schema.contains(r#""count":{"type":"integer""#));
//!
//! let argv = tool::to_argv(&cmd, "greet", r#"{"count": 2, "loud": true, "FILE": "x"}"#).unwrap();
//! assert_eq!(argv, ["--count=2", "--loud", "x"]);
//! ```
//!
//! Property names are the argument ids. Flags map to booleans, counters
//! to integers, values to the JSON type matching their Rust type
//! ([`ValueType`]), `possible` values to `enum`, repeated arguments to
//! arrays. Hidden arguments are left out. Global options of enclosing
//! commands are included in every subcommand's tool.

use std::ffi::OsString;

use hasami::{ArgDef, Command, ValueType};

/// One callable tool: a command (or subcommand) with an input schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tool {
    /// `app`, `app_add`, ...: the command path with non-alphanumeric
    /// characters replaced by `_`.
    pub name: String,
    /// The command path as words: `["app", "add"]`.
    pub path: Vec<String>,
    /// The command's description (`long_about`, else `about`).
    pub description: String,
    /// A JSON Schema object for the input.
    pub input_schema: String,
}

impl Tool {
    /// The tool in the shape Claude's API takes: `name`, `description`,
    /// `input_schema`.
    pub fn to_json(&self) -> String {
        self.render("input_schema")
    }

    /// The tool in the shape the Model Context Protocol takes: `name`,
    /// `description`, `inputSchema`.
    pub fn to_mcp_json(&self) -> String {
        self.render("inputSchema")
    }

    fn render(&self, schema_key: &str) -> String {
        let mut s = String::from("{\"name\":");
        json_str(&mut s, &self.name);
        s.push_str(",\"description\":");
        json_str(&mut s, &self.description);
        s.push_str(",\"");
        s.push_str(schema_key);
        s.push_str("\":");
        s.push_str(&self.input_schema);
        s.push('}');
        s
    }
}

/// Every tool of `cmd`, root first, then subcommands depth first.
pub fn tools(cmd: &Command) -> Vec<Tool> {
    let mut out = Vec::new();
    collect(cmd, vec![cmd.name().to_owned()], &[], &mut out);
    out
}

/// A JSON array of every tool, Claude style. `mcp` selects the MCP
/// spelling of the schema key.
pub fn to_json(tools: &[Tool], mcp: bool) -> String {
    let mut s = String::from("[");
    for (i, t) in tools.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&if mcp { t.to_mcp_json() } else { t.to_json() });
    }
    s.push(']');
    s
}

/// The arguments visible in a tool at `path`: inherited globals first.
fn visible_args<'a>(cmd: &'a Command, inherited: &[&'a ArgDef]) -> Vec<&'a ArgDef> {
    let mut args: Vec<&ArgDef> = inherited.to_vec();
    args.extend(cmd.args().iter().filter(|a| !a.is_hidden()));
    args
}

fn collect<'a>(cmd: &'a Command, path: Vec<String>, inherited: &[&'a ArgDef], out: &mut Vec<Tool>) {
    let runnable = cmd.subcommands().is_empty() || !cmd.is_subcommand_required();
    if runnable {
        let args = visible_args(cmd, inherited);
        let description = cmd
            .get_long_about()
            .or(cmd.get_about())
            .map(str::to_owned)
            .unwrap_or_else(|| ["Run `", &path.join(" "), "`"].concat());
        out.push(Tool {
            name: tool_name(&path),
            path: path.clone(),
            description,
            input_schema: input_schema(&args),
        });
    }
    // Subcommands see the globals of every enclosing command.
    let mut globals: Vec<&ArgDef> = inherited.to_vec();
    globals.extend(
        cmd.args()
            .iter()
            .filter(|a| a.is_global() && !a.is_hidden()),
    );
    let built: Vec<(String, Command)> = cmd
        .subcommands()
        .iter()
        .filter(|s| !s.is_hidden())
        .map(|s| (s.name().to_owned(), s.build()))
        .collect();
    for (name, sub) in &built {
        let mut p = path.clone();
        p.push(name.clone());
        // Borrow the built subcommand for the recursion only; its args
        // are not inherited further up, so the lifetime stays local.
        let mut local: Vec<Tool> = Vec::new();
        collect_owned(sub, p, &globals, &mut local);
        out.extend(local);
    }
}

/// `collect` for a subcommand that was built (owned) on the way down.
fn collect_owned(cmd: &Command, path: Vec<String>, inherited: &[&ArgDef], out: &mut Vec<Tool>) {
    let runnable = cmd.subcommands().is_empty() || !cmd.is_subcommand_required();
    if runnable {
        let args = visible_args(cmd, inherited);
        let description = cmd
            .get_long_about()
            .or(cmd.get_about())
            .map(str::to_owned)
            .unwrap_or_else(|| ["Run `", &path.join(" "), "`"].concat());
        out.push(Tool {
            name: tool_name(&path),
            path: path.clone(),
            description,
            input_schema: input_schema(&args),
        });
    }
    let mut globals: Vec<&ArgDef> = inherited.to_vec();
    globals.extend(
        cmd.args()
            .iter()
            .filter(|a| a.is_global() && !a.is_hidden()),
    );
    for s in cmd.subcommands() {
        if s.is_hidden() {
            continue;
        }
        let mut p = path.clone();
        p.push(s.name().to_owned());
        collect_owned(&s.build(), p, &globals, out);
    }
}

fn tool_name(path: &[String]) -> String {
    path.join("_")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// The JSON Schema `type` for a value.
fn json_type(a: &ArgDef) -> &'static str {
    match a.value_type() {
        Some(ValueType::Integer) => "integer",
        Some(ValueType::Float) => "number",
        Some(ValueType::Boolean) => "boolean",
        _ => "string",
    }
}

fn input_schema(args: &[&ArgDef]) -> String {
    let mut s = String::from("{\"type\":\"object\",\"properties\":{");
    let mut required = Vec::new();
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        json_str(&mut s, a.id());
        s.push(':');
        property(&mut s, a);
        if a.is_required() {
            required.push(a.id());
        }
    }
    s.push_str("},\"required\":[");
    for (i, r) in required.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        json_str(&mut s, r);
    }
    s.push_str("],\"additionalProperties\":false}");
    s
}

fn property(s: &mut String, a: &ArgDef) {
    s.push('{');
    let base_type = if a.is_count() {
        "integer"
    } else if !a.takes_value() {
        "boolean"
    } else {
        json_type(a)
    };
    let mut item = String::new();
    item.push_str("\"type\":\"");
    item.push_str(base_type);
    item.push('"');
    if !a.possible_values().is_empty() {
        item.push_str(",\"enum\":[");
        for (i, v) in a.possible_values().iter().enumerate() {
            if i > 0 {
                item.push(',');
            }
            json_str(&mut item, v);
        }
        item.push(']');
    }
    if a.is_many() {
        s.push_str("\"type\":\"array\",\"items\":{");
        s.push_str(&item);
        s.push('}');
    } else {
        s.push_str(&item);
    }
    if a.is_count() {
        s.push_str(",\"minimum\":0");
    }
    let mut description = a.long_help().or(a.help()).unwrap_or("").to_owned();
    #[cfg(feature = "env")]
    if let Some(var) = a.env() {
        if !description.is_empty() {
            description.push(' ');
        }
        description.push_str("[env: ");
        description.push_str(var);
        description.push(']');
    }
    if a.value_is_optional() {
        if !description.is_empty() {
            description.push(' ');
        }
        description.push_str("[true selects the default value: ");
        description.push_str(a.default_missing_text().unwrap_or(""));
        description.push(']');
    }
    if !description.is_empty() {
        s.push_str(",\"description\":");
        json_str(s, &description);
    }
    if let Some(d) = a.default_text() {
        s.push_str(",\"default\":");
        match base_type {
            "integer" | "number" if d.parse::<f64>().is_ok() => s.push_str(d),
            "boolean" if d == "true" || d == "false" => s.push_str(d),
            _ => json_str(s, d),
        }
    }
    s.push('}');
}

/// Turn an agent's JSON input for `tool_name` into the argument vector
/// (without the binary name) and check it by parsing with `cmd`.
///
/// Options are written as `--long=value` (or `-s value` when there is
/// no long name), flags as `--long`, counters repeated, arrays as one
/// occurrence per element, positionals last in definition order after
/// `--` when one of them starts with `-`. A `true` for an option with an
/// optional value selects its `default_missing`.
///
/// # Errors
///
/// A message when the JSON is malformed, the tool is unknown, a key does
/// not name an argument, a value has the wrong shape, or the resulting
/// command line does not parse (the parser's own message).
pub fn to_argv(cmd: &Command, tool_name: &str, input: &str) -> Result<Vec<OsString>, String> {
    let tool = tools(cmd)
        .into_iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| ["unknown tool '", tool_name, "'"].concat())?;
    let value = json::parse(input)?;
    let json::Value::Object(fields) = value else {
        return Err("tool input must be a JSON object".to_owned());
    };
    // Rebuild the argument list this tool exposes.
    let mut node = cmd.clone();
    let mut visible: Vec<ArgDef> = Vec::new();
    for word in &tool.path[1..] {
        visible.extend(
            node.args()
                .iter()
                .filter(|a| a.is_global() && !a.is_hidden())
                .cloned(),
        );
        node = node
            .find_subcommand(word)
            .ok_or_else(|| ["unknown subcommand '", word, "'"].concat())?
            .build();
    }
    visible.extend(node.args().iter().filter(|a| !a.is_hidden()).cloned());

    let mut argv: Vec<OsString> = tool.path[1..].iter().map(OsString::from).collect();
    let mut positionals: Vec<(usize, Vec<String>)> = Vec::new();
    for (key, value) in &fields {
        let (index, def) = visible
            .iter()
            .enumerate()
            .find(|(_, a)| a.id() == key)
            .ok_or_else(|| ["unknown argument '", key, "'"].concat())?;
        let values = scalar_values(key, value, def)?;
        if def.is_positional() {
            positionals.push((index, values));
            continue;
        }
        if def.is_count() {
            let n: usize = values
                .first()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| ["'", key, "' must be a non-negative integer"].concat())?;
            for _ in 0..n {
                argv.push(option_name(def));
            }
            continue;
        }
        if !def.takes_value() {
            if values.first().map(String::as_str) == Some("true") {
                argv.push(option_name(def));
            }
            continue;
        }
        for v in values {
            if def.value_is_optional() && v == "true" {
                argv.push(option_name(def));
            } else if let Some(long) = def.long() {
                argv.push(OsString::from(["--", long, "=", &v].concat()));
            } else {
                argv.push(option_name(def));
                argv.push(OsString::from(v));
            }
        }
    }
    positionals.sort_by_key(|(i, _)| *i);
    let values: Vec<String> = positionals.into_iter().flat_map(|(_, v)| v).collect();
    if values.iter().any(|v| v.starts_with('-')) {
        argv.push(OsString::from("--"));
    }
    argv.extend(values.into_iter().map(OsString::from));
    cmd.try_parse_args(argv.clone())
        .map_err(|e| e.message().to_owned())?;
    Ok(argv)
}

fn option_name(def: &ArgDef) -> OsString {
    match (def.long(), def.short()) {
        (Some(l), _) => OsString::from(["--", l].concat()),
        (None, Some(c)) => OsString::from(["-", &c.to_string()].concat()),
        (None, None) => OsString::from(def.id()),
    }
}

/// The JSON value as one or more strings, checking its shape against
/// the argument.
fn scalar_values(key: &str, value: &json::Value, def: &ArgDef) -> Result<Vec<String>, String> {
    use json::Value;
    let one = |v: &Value| -> Result<String, String> {
        match v {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.clone()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Null => Err(["'", key, "' is null"].concat()),
            Value::Array(_) => Err(["'", key, "' does not take a list"].concat()),
            Value::Object(_) => Err(["'", key, "' does not take an object"].concat()),
        }
    };
    match value {
        Value::Array(items) if def.is_many() => items.iter().map(one).collect(),
        Value::Array(_) => Err(["'", key, "' does not take a list"].concat()),
        Value::Null => Ok(Vec::new()),
        other => Ok(vec![one(other)?]),
    }
}

/// Append `s` as a JSON string literal.
pub(crate) fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A small JSON reader, enough for tool inputs. Numbers are kept as
/// their source text.
mod json {
    pub enum Value {
        Null,
        Bool(bool),
        Number(String),
        String(String),
        Array(Vec<Value>),
        Object(Vec<(String, Value)>),
    }

    pub fn parse(text: &str) -> Result<Value, String> {
        let mut p = Parser {
            s: text.as_bytes(),
            i: 0,
        };
        let v = p.value()?;
        p.ws();
        if p.i != p.s.len() {
            return Err(p.err("trailing characters"));
        }
        Ok(v)
    }

    struct Parser<'a> {
        s: &'a [u8],
        i: usize,
    }

    impl Parser<'_> {
        fn err(&self, what: &str) -> String {
            format!("invalid JSON at byte {}: {what}", self.i)
        }

        fn ws(&mut self) {
            while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
                self.i += 1;
            }
        }

        fn eat(&mut self, lit: &str) -> bool {
            if self.s[self.i..].starts_with(lit.as_bytes()) {
                self.i += lit.len();
                true
            } else {
                false
            }
        }

        fn value(&mut self) -> Result<Value, String> {
            self.ws();
            match self.s.get(self.i) {
                None => Err(self.err("unexpected end")),
                Some(b'{') => self.object(),
                Some(b'[') => self.array(),
                Some(b'"') => self.string().map(Value::String),
                Some(b't') if self.eat("true") => Ok(Value::Bool(true)),
                Some(b'f') if self.eat("false") => Ok(Value::Bool(false)),
                Some(b'n') if self.eat("null") => Ok(Value::Null),
                Some(c) if *c == b'-' || c.is_ascii_digit() => self.number(),
                Some(_) => Err(self.err("unexpected character")),
            }
        }

        fn object(&mut self) -> Result<Value, String> {
            self.i += 1;
            let mut fields = Vec::new();
            self.ws();
            if self.eat("}") {
                return Ok(Value::Object(fields));
            }
            loop {
                self.ws();
                if self.s.get(self.i) != Some(&b'"') {
                    return Err(self.err("expected a key"));
                }
                let key = self.string()?;
                self.ws();
                if !self.eat(":") {
                    return Err(self.err("expected ':'"));
                }
                let v = self.value()?;
                fields.push((key, v));
                self.ws();
                if self.eat(",") {
                    continue;
                }
                if self.eat("}") {
                    return Ok(Value::Object(fields));
                }
                return Err(self.err("expected ',' or '}'"));
            }
        }

        fn array(&mut self) -> Result<Value, String> {
            self.i += 1;
            let mut items = Vec::new();
            self.ws();
            if self.eat("]") {
                return Ok(Value::Array(items));
            }
            loop {
                items.push(self.value()?);
                self.ws();
                if self.eat(",") {
                    continue;
                }
                if self.eat("]") {
                    return Ok(Value::Array(items));
                }
                return Err(self.err("expected ',' or ']'"));
            }
        }

        fn number(&mut self) -> Result<Value, String> {
            let start = self.i;
            while self.i < self.s.len()
                && matches!(
                    self.s[self.i],
                    b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'
                )
            {
                self.i += 1;
            }
            let text =
                std::str::from_utf8(&self.s[start..self.i]).map_err(|_| self.err("bad number"))?;
            if text.parse::<f64>().is_err() {
                return Err(self.err("bad number"));
            }
            Ok(Value::Number(text.to_owned()))
        }

        fn string(&mut self) -> Result<String, String> {
            self.i += 1;
            let mut out = String::new();
            let mut pending_high: Option<u32> = None;
            loop {
                let Some(&b) = self.s.get(self.i) else {
                    return Err(self.err("unterminated string"));
                };
                self.i += 1;
                match b {
                    b'"' => return Ok(out),
                    b'\\' => {
                        let Some(&e) = self.s.get(self.i) else {
                            return Err(self.err("unterminated escape"));
                        };
                        self.i += 1;
                        match e {
                            b'"' => out.push('"'),
                            b'\\' => out.push('\\'),
                            b'/' => out.push('/'),
                            b'b' => out.push('\u{8}'),
                            b'f' => out.push('\u{c}'),
                            b'n' => out.push('\n'),
                            b'r' => out.push('\r'),
                            b't' => out.push('\t'),
                            b'u' => {
                                let hex = self
                                    .s
                                    .get(self.i..self.i + 4)
                                    .and_then(|h| std::str::from_utf8(h).ok())
                                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                                    .ok_or_else(|| self.err("bad \\u escape"))?;
                                self.i += 4;
                                let code = match (pending_high.take(), hex) {
                                    (Some(hi), lo @ 0xDC00..=0xDFFF) => {
                                        0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                                    }
                                    (None, hi @ 0xD800..=0xDBFF) => {
                                        pending_high = Some(hi);
                                        continue;
                                    }
                                    (_, c) => c,
                                };
                                out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                            }
                            _ => return Err(self.err("bad escape")),
                        }
                    }
                    _ => {
                        // Copy one UTF-8 sequence.
                        let start = self.i - 1;
                        let len = match b {
                            0x00..=0x7F => 1,
                            0xC0..=0xDF => 2,
                            0xE0..=0xEF => 3,
                            _ => 4,
                        };
                        let end = (start + len).min(self.s.len());
                        let piece = std::str::from_utf8(&self.s[start..end])
                            .map_err(|_| self.err("bad UTF-8"))?;
                        out.push_str(piece);
                        self.i = end;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hasami::{Arg, Command, Subcommand};

    fn app() -> Command {
        let verbose = Arg::new("verbose").short('v').count().global().help("More");
        let color = Arg::new("color")
            .value::<String>()
            .possible(["auto", "always", "never"])
            .default("auto".to_owned())
            .help("When");
        let secret = Arg::new("secret").hidden();
        let name = Arg::positional::<String>("NAME").help("Who");
        let tags = Arg::new("tag")
            .short('t')
            .value::<String>()
            .many()
            .help("Tags");
        let ratio = Arg::new("ratio").value::<f64>().help("Mix");
        let force = Arg::new("force").short('f').help("Force");
        let files = Arg::positional_os("FILE").many().help("Files");
        Command::new("app")
            .about("An app")
            .arg(&verbose)
            .arg(&color)
            .arg(&secret)
            .arg(&name)
            .arg(&tags)
            .subcommand(
                Command::new("add")
                    .long_about("Add things.\nAt length.")
                    .arg(&ratio)
                    .arg(&force)
                    .arg(&files),
            )
            .subcommand(Subcommand::from(Command::new("hidden")).hidden())
    }

    #[test]
    fn one_tool_per_runnable_command() {
        let tools = tools(&app());
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["app", "app_add"]);
        assert_eq!(tools[1].path, ["app", "add"]);
        assert_eq!(tools[1].description, "Add things.\nAt length.");
        let root = &tools[0].input_schema;
        assert!(root.starts_with(r#"{"type":"object","properties":{"verbose":{"type":"integer","minimum":0,"description":"More"},"color":{"type":"string","enum":["auto","always","never"],"description":"When","default":"auto"},"NAME":{"type":"string","description":"Who"},"tag":{"type":"array","items":{"type":"string"},"description":"Tags"}},"required":[],"additionalProperties":false}"#), "{root}");
        assert!(!root.contains("secret"));
        let add = &tools[1].input_schema;
        assert!(add.contains(r#""verbose":{"type":"integer""#), "{add}");
        assert!(
            add.contains(r#""ratio":{"type":"number","description":"Mix"}"#),
            "{add}"
        );
        assert!(
            add.contains(r#""force":{"type":"boolean","description":"Force"}"#),
            "{add}"
        );
        assert!(
            add.contains(
                r#""FILE":{"type":"array","items":{"type":"string"},"description":"Files"}"#
            ),
            "{add}"
        );
        assert!(
            tools[0]
                .to_json()
                .starts_with(r#"{"name":"app","description":"An app","input_schema":{"#)
        );
        assert!(tools[0].to_mcp_json().contains(r#""inputSchema":{"#));
        assert!(to_json(&tools, false).starts_with("[{\"name\":\"app\""));
    }

    #[test]
    fn required_subcommand_root_is_not_a_tool() {
        let cmd = Command::new("x")
            .subcommand(Command::new("a"))
            .subcommand_required();
        let names: Vec<String> = tools(&cmd).into_iter().map(|t| t.name).collect();
        assert_eq!(names, ["x_a"]);
    }

    #[test]
    fn to_argv_round_trips() {
        let cmd = app();
        let argv = to_argv(
            &cmd,
            "app",
            r#"{"verbose": 2, "color": "never", "NAME": "bob", "tag": ["a", "b"]}"#,
        )
        .unwrap();
        assert_eq!(
            argv,
            [
                "--verbose",
                "--verbose",
                "--color=never",
                "--tag=a",
                "--tag=b",
                "bob"
            ]
        );
        let argv = to_argv(
            &cmd,
            "app_add",
            r#"{"verbose": 1, "ratio": 0.5, "force": true, "FILE": ["-x", "y"]}"#,
        )
        .unwrap();
        assert_eq!(
            argv,
            [
                "add",
                "--verbose",
                "--ratio=0.5",
                "--force",
                "--",
                "-x",
                "y"
            ]
        );
        let argv = to_argv(&cmd, "app_add", r#"{"force": false}"#).unwrap();
        assert_eq!(argv, ["add"]);
    }

    #[test]
    fn to_argv_reports_problems() {
        let cmd = app();
        assert_eq!(
            to_argv(&cmd, "nope", "{}").unwrap_err(),
            "unknown tool 'nope'"
        );
        assert_eq!(
            to_argv(&cmd, "app", r#"{"bogus": 1}"#).unwrap_err(),
            "unknown argument 'bogus'"
        );
        assert_eq!(
            to_argv(&cmd, "app", r#"{"NAME": ["a"]}"#).unwrap_err(),
            "'NAME' does not take a list"
        );
        assert!(
            to_argv(&cmd, "app", r#"{"NAME": "x", "color": "sometimes"}"#)
                .unwrap_err()
                .starts_with("invalid value 'sometimes'")
        );
        assert!(
            to_argv(&cmd, "app", "[1]")
                .unwrap_err()
                .contains("must be a JSON object")
        );
        assert!(
            to_argv(&cmd, "app", r#"{"NAME": "x""#)
                .unwrap_err()
                .starts_with("invalid JSON")
        );
        // Parser errors are passed through.
        let req = Arg::positional::<String>("NAME").required();
        let strict = Command::new("s").arg(&req);
        assert!(to_argv(&strict, "s", "{}").unwrap_err().contains("<NAME>"));
        let strict_tools = tools(&strict);
        assert!(
            strict_tools[0]
                .input_schema
                .contains(r#""required":["NAME"]"#)
        );
    }

    #[test]
    fn json_reader_handles_escapes_and_unicode() {
        let v = json::parse(r#"{"s": "a\"b\\c\né😀", "n": -1.5e3, "t": true, "z": null}"#).unwrap();
        let json::Value::Object(f) = v else { panic!() };
        assert!(matches!(&f[0].1, json::Value::String(s) if s == "a\"b\\c\né😀"));
        assert!(matches!(&f[1].1, json::Value::Number(n) if n == "-1.5e3"));
        assert!(matches!(f[2].1, json::Value::Bool(true)));
        assert!(matches!(f[3].1, json::Value::Null));
        assert!(json::parse("{").is_err());
        assert!(json::parse("{} x").is_err());
    }
}
