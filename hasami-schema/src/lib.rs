//! A machine-readable JSON description of a [`hasami::Command`]: the exact
//! set of arguments, subcommands and constraints the parser accepts,
//! **including hidden ones**. Help settings do not affect it.
//!
//! The document format is described by [`JSON_SCHEMA`], a JSON Schema
//! (draft 2020-12) that consumers can validate against.
//!
//! No dependencies: the JSON is written by hand with proper escaping.
//!
//! ```
//! use hasami::{Arg, Command};
//!
//! let n = Arg::new("number").short('n').value::<u32>().default(1).help("How many");
//! let cmd = Command::new("app").version("1.0").arg(&n);
//! let json = hasami_schema::to_json(&cmd);
//! assert!(json.contains(r#""id":"number""#));
//! assert!(json.contains(r#""default":"1""#));
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use hasami::{ArgDef, Command, Relation, ValueType};

pub mod tool;

/// JSON Schema for the documents produced by [`to_json`].
pub const JSON_SCHEMA: &str = include_str!("schema.json");

/// The version of the document format, written into every document as
/// `"format"`.
pub const FORMAT_VERSION: u32 = 2;

/// Render `cmd` (and every subcommand, built if lazy) as compact JSON.
pub fn to_json(cmd: &Command) -> String {
    let mut w = Writer::new(false);
    w.command(cmd, true);
    w.out
}

/// Like [`to_json`], indented with two spaces per level.
pub fn to_json_pretty(cmd: &Command) -> String {
    let mut w = Writer::new(true);
    w.command(cmd, true);
    w.out.push('\n');
    w.out
}

struct Writer {
    out: String,
    pretty: bool,
    depth: usize,
}

impl Writer {
    fn new(pretty: bool) -> Writer {
        Writer {
            out: String::new(),
            pretty,
            depth: 0,
        }
    }

    fn newline(&mut self) {
        if self.pretty {
            self.out.push('\n');
            for _ in 0..self.depth {
                self.out.push_str("  ");
            }
        }
    }

    fn object(&mut self, f: impl FnOnce(&mut Object<'_>)) {
        self.out.push('{');
        self.depth += 1;
        let mut obj = Object {
            w: self,
            first: true,
        };
        f(&mut obj);
        let first = obj.first;
        self.depth -= 1;
        if !first {
            self.newline();
        }
        self.out.push('}');
    }

    fn array<T>(&mut self, items: impl IntoIterator<Item = T>, mut f: impl FnMut(&mut Writer, T)) {
        self.out.push('[');
        self.depth += 1;
        let mut first = true;
        for item in items {
            if !first {
                self.out.push(',');
            }
            first = false;
            self.newline();
            f(self, item);
        }
        self.depth -= 1;
        if !first {
            self.newline();
        }
        self.out.push(']');
    }

    fn string(&mut self, s: &str) {
        self.out.push('"');
        for c in s.chars() {
            match c {
                '"' => self.out.push_str("\\\""),
                '\\' => self.out.push_str("\\\\"),
                '\n' => self.out.push_str("\\n"),
                '\r' => self.out.push_str("\\r"),
                '\t' => self.out.push_str("\\t"),
                c if (c as u32) < 0x20 => {
                    let mut buf = [0u8; 4];
                    let hex = format_hex(c as u32, &mut buf);
                    self.out.push_str("\\u00");
                    self.out.push_str(hex);
                }
                c => self.out.push(c),
            }
        }
        self.out.push('"');
    }

    fn opt_string(&mut self, s: Option<&str>) {
        match s {
            Some(s) => self.string(s),
            None => self.out.push_str("null"),
        }
    }

    fn bool(&mut self, b: bool) {
        self.out.push_str(if b { "true" } else { "false" });
    }

    fn command(&mut self, cmd: &Command, root: bool) {
        self.object(|o| {
            if root {
                o.key("format");
                o.w.out.push_str(&FORMAT_VERSION.to_string());
            }
            o.key("name");
            o.w.string(cmd.name());
            o.key("version");
            o.w.opt_string(cmd.get_version());
            o.key("about");
            o.w.opt_string(cmd.get_about());
            o.key("long_about");
            o.w.opt_string(cmd.get_long_about());
            o.key("after_help");
            o.w.opt_string(cmd.get_after_help());
            o.key("before_help");
            o.w.opt_string(cmd.get_before_help());
            o.key("long_version");
            o.w.opt_string(cmd.get_long_version());
            o.key("args_override_self");
            o.w.bool(cmd.is_args_override_self());
            o.key("arg_required_else_help");
            o.w.bool(cmd.is_arg_required_else_help());
            o.key("infer_long_args");
            o.w.bool(cmd.is_infer_long_args());
            o.key("infer_subcommands");
            o.w.bool(cmd.is_infer_subcommands());
            o.key("external_subcommands");
            o.w.bool(cmd.allows_external_subcommands());
            o.key("examples");
            o.w.array(cmd.get_examples(), |w, e| w.string(e));
            o.key("help_flag");
            o.w.bool(cmd.has_help_flag());
            o.key("version_flag");
            o.w.bool(cmd.has_version_flag());
            o.key("subcommand_required");
            o.w.bool(cmd.is_subcommand_required());
            o.key("args");
            o.w.array(cmd.args(), |w, a| w.arg(a));
            o.key("groups");
            o.w.array(cmd.groups(), |w, g| {
                w.object(|o| {
                    o.key("name");
                    o.w.string(g.name());
                    o.key("members");
                    o.w.array(g.members(), |w, m| w.string(m));
                    o.key("required");
                    o.w.bool(g.is_required());
                    o.key("exclusive");
                    o.w.bool(g.is_exclusive());
                });
            });
            o.key("requires");
            o.w.array(cmd.requirements(), |w, (a, b)| {
                w.array([a, b], |w, s| w.string(s));
            });
            o.key("subcommands");
            o.w.array(cmd.subcommands(), |w, s| {
                w.object(|o| {
                    o.key("name");
                    o.w.string(s.name());
                    o.key("aliases");
                    o.w.array(s.aliases(), |w, a| w.string(a));
                    o.key("visible_aliases");
                    o.w.array(s.visible_aliases(), |w, a| w.string(a));
                    o.key("hidden");
                    o.w.bool(s.is_hidden());
                    o.key("about");
                    o.w.opt_string(s.summary());
                    o.key("command");
                    let built = s.build();
                    o.w.command(&built, false);
                });
            });
        });
    }

    fn arg(&mut self, a: &ArgDef) {
        self.object(|o| {
            o.key("id");
            o.w.string(a.id());
            o.key("long");
            o.w.opt_string(a.long());
            o.key("short");
            match a.short() {
                Some(c) => o.w.string(&c.to_string()),
                None => o.w.out.push_str("null"),
            }
            o.key("aliases");
            o.w.array(a.aliases(), |w, s| w.string(s));
            o.key("visible_aliases");
            o.w.array(a.visible_aliases(), |w, s| w.string(s));
            o.key("short_aliases");
            o.w.array(a.short_aliases(), |w, c| w.string(&c.to_string()));
            o.key("positional");
            o.w.bool(a.is_positional());
            o.key("takes_value");
            o.w.bool(a.takes_value());
            o.key("value_name");
            o.w.opt_string(a.value_name());
            o.key("value_type");
            o.w.opt_string(a.value_type().map(value_type_name));
            o.key("value_optional");
            o.w.bool(a.value_is_optional());
            o.key("required");
            o.w.bool(a.is_required());
            o.key("many");
            o.w.bool(a.is_many());
            o.key("count");
            o.w.bool(a.is_count());
            o.key("last_wins");
            o.w.bool(a.is_last_wins());
            o.key("greedy");
            o.w.bool(a.is_greedy());
            o.key("trailing");
            o.w.bool(a.is_trailing());
            o.key("delimiter");
            match a.delimiter() {
                Some(c) => o.w.string(&c.to_string()),
                None => o.w.out.push_str("null"),
            }
            o.key("hidden");
            o.w.bool(a.is_hidden());
            o.key("global");
            o.w.bool(a.is_global());
            o.key("help");
            o.w.opt_string(a.help());
            o.key("long_help");
            o.w.opt_string(a.long_help());
            o.key("help_heading");
            o.w.opt_string(a.help_heading());
            o.key("default");
            o.w.opt_string(a.default_text());
            o.key("default_missing");
            o.w.opt_string(a.default_missing_text());
            o.key("possible_values");
            o.w.array(a.possible_values(), |w, s| w.string(s));
            o.key("env");
            #[cfg(feature = "env")]
            o.w.opt_string(a.env());
            #[cfg(not(feature = "env"))]
            o.w.opt_string(None);
            o.key("dynamic_completion");
            o.w.bool(a.completer().is_some());
            let rel = |o: &mut Object<'_>, key: &str, f: fn(&Relation) -> Option<Vec<&str>>| {
                o.key(key);
                let items: Vec<Vec<&str>> = a.relations().iter().filter_map(f).collect();
                o.w.array(&items, |w, parts| {
                    if let [one] = parts.as_slice() {
                        w.string(one);
                    } else {
                        w.array(parts, |w, s| w.string(s));
                    }
                });
            };
            rel(o, "requires", |r| match r {
                Relation::Requires(id) => Some(vec![id]),
                _ => None,
            });
            rel(o, "conflicts_with", |r| match r {
                Relation::ConflictsWith(id) => Some(vec![id]),
                _ => None,
            });
            rel(o, "required_unless", |r| match r {
                Relation::RequiredUnless(id) => Some(vec![id]),
                _ => None,
            });
            rel(o, "required_if_eq", |r| match r {
                Relation::RequiredIfEq(id, v) => Some(vec![id, v]),
                _ => None,
            });
            rel(o, "requires_if", |r| match r {
                Relation::RequiresIf(v, id) => Some(vec![v, id]),
                _ => None,
            });
        });
    }
}

/// The JSON name of a [`ValueType`].
pub(crate) fn value_type_name(t: ValueType) -> &'static str {
    match t {
        ValueType::Integer => "integer",
        ValueType::Float => "float",
        ValueType::Boolean => "boolean",
        ValueType::String => "string",
        ValueType::Path => "path",
        _ => "other",
    }
}

struct Object<'a> {
    w: &'a mut Writer,
    first: bool,
}

impl Object<'_> {
    fn key(&mut self, k: &str) {
        if !self.first {
            self.w.out.push(',');
        }
        self.first = false;
        self.w.newline();
        self.w.string(k);
        self.w.out.push(':');
        if self.w.pretty {
            self.w.out.push(' ');
        }
    }
}

fn format_hex(v: u32, buf: &mut [u8; 4]) -> &str {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    buf[0] = HEX[((v >> 4) & 0xf) as usize];
    buf[1] = HEX[(v & 0xf) as usize];
    // Only ASCII hex digits were written.
    std::str::from_utf8(&buf[..2]).unwrap_or("00")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hasami::{Arg, Group, Subcommand};

    fn sample() -> Command {
        let n = Arg::new("number")
            .short('n')
            .value::<u32>()
            .default(1)
            .help("How\nmany \"times\"");
        let mode = Arg::new("mode")
            .value::<String>()
            .possible(["a", "b"])
            .hidden();
        let json = Arg::new("json");
        let yaml = Arg::new("yaml");
        let file = Arg::positional::<String>("FILE")
            .complete_with(|_| vec!["x".into()])
            .many();
        Command::new("app")
            .version("1.0")
            .about("About")
            .arg(&n)
            .arg(&mode)
            .arg(&json)
            .arg(&yaml)
            .arg(&file)
            .group(Group::new("fmt").member(&json).member(&yaml).exclusive())
            .requires(&json, &n)
            .subcommand(Command::new("sub").about("Sub").arg(&n))
            .subcommand(
                Subcommand::lazy("lazy", || Command::new("lazy"))
                    .alias("l")
                    .hidden(),
            )
    }

    #[test]
    fn compact_document_shape() {
        let json = to_json(&sample());
        assert!(json.starts_with(r#"{"format":2,"name":"app","version":"1.0","about":"About","#));
        assert!(json.contains(r#""help":"How\nmany \"times\"""#));
        assert!(json.contains(r#""id":"mode","long":"mode","short":null,"aliases":[],"visible_aliases":[],"short_aliases":[],"positional":false,"takes_value":true,"value_name":"MODE","value_type":"string","value_optional":false,"required":false,"many":false,"count":false,"last_wins":false,"greedy":false,"trailing":false,"delimiter":null,"hidden":true,"global":false,"help":null,"long_help":null,"help_heading":null,"default":null,"default_missing":null,"possible_values":["a","b"],"env":null,"dynamic_completion":false,"requires":[],"conflicts_with":[],"required_unless":[],"required_if_eq":[],"requires_if":[]}"#));
        assert!(json.contains(r#""groups":[{"name":"fmt","members":["json","yaml"],"required":false,"exclusive":true}]"#));
        assert!(json.contains(r#""requires":[["json","number"]]"#));
        assert!(json.contains(
            r#""name":"lazy","aliases":["l"],"visible_aliases":[],"hidden":true,"about":null,"command":{"name":"lazy""#
        ));
        assert!(json.contains(r#""id":"FILE","long":null,"short":null,"aliases":[],"visible_aliases":[],"short_aliases":[],"positional":true"#));
        assert!(json.contains(r#""dynamic_completion":true"#));
        // Nested commands carry no "format" key.
        assert_eq!(json.matches("\"format\"").count(), 1);
    }

    #[test]
    fn pretty_is_same_data() {
        let compact = to_json(&sample());
        let pretty = to_json_pretty(&sample());
        let squeezed: String = pretty
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("")
            .replace("\": ", "\":");
        assert_eq!(squeezed, compact);
    }

    #[test]
    fn escapes_control_characters() {
        let a = Arg::new("x").help("tab\there\u{1}");
        let json = to_json(&Command::new("c").arg(&a));
        assert!(json.contains(r#""help":"tab\there\u0001""#));
    }

    #[test]
    fn schema_document_is_present() {
        assert!(JSON_SCHEMA.contains("\"$schema\""));
        assert!(JSON_SCHEMA.contains("\"dynamic_completion\""));
    }
}
