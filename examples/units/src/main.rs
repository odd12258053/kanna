//! `units`: a unit converter that an AI agent can drive. The definition
//! is written once with the `cli!` macro; `kanna-schema` derives the
//! machine-readable side from it. `tools` prints Claude / MCP tool
//! definitions, `call` runs a tool the way an agent would (JSON input →
//! argument vector → parse → run), and `schema` prints the complete JSON
//! description. With `KANNA_ERROR_FORMAT=json` every error is one JSON
//! object on stderr, so a driver can read it back.
//!
//! ```text
//! cargo run -p kanna-example-units -- length 5 --from km --to mi
//! cargo run -p kanna-example-units -- temp -f c -t f -- -40
//! cargo run -p kanna-example-units -- --precision 0 mass 3 --from lb --to g
//! cargo run -p kanna-example-units -- tools
//! cargo run -p kanna-example-units -- tools --mcp
//! cargo run -p kanna-example-units -- call units_temp '{"value": 100, "from": "c", "to": "f"}'
//! cargo run -p kanna-example-units -- call --dry-run units_length '{"value": 1, "from": "mi", "to": "m", "precision": 0}'
//! cargo run -p kanna-example-units -- schema
//! KANNA_ERROR_FORMAT=json cargo run -q -p kanna-example-units -- call units_temp '{"value": "hot", "from": "c", "to": "f"}'
//! ```
//!
//! The tool names are the command paths joined with `_`: `units_length`,
//! `units_mass` and `units_temp`. The `tools`, `call` and `schema`
//! subcommands are marked `no_tool`: they stay in `--help` but are not
//! offered to the agent, which would otherwise see the driver's own
//! commands and could call `call` recursively. The property names are the
//! argument ids (the field names below); the global `--precision` appears
//! in every tool. Doc comments on the value enums' variants become the
//! legend of the unit properties (`c = Celsius, ...`), and `long_about`
//! on each converter tells the agent what the output line looks like.
#![forbid(unsafe_code)]

use std::ffi::OsString;

use kanna::{Cli, Error, ValueEnum};
use kanna_schema::tool;

kanna::value_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Length {
        /// millimetre
        Millimetre = "mm",
        /// centimetre
        Centimetre = "cm",
        /// metre
        Metre = "m",
        /// kilometre
        Kilometre = "km",
        /// inch
        Inch = "in",
        /// foot
        Foot = "ft",
        /// mile
        Mile = "mi",
    }
}

kanna::value_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Mass {
        /// gram
        Gram = "g",
        /// kilogram
        Kilogram = "kg",
        /// ounce
        Ounce = "oz",
        /// pound
        Pound = "lb",
    }
}

kanna::value_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Temperature {
        /// Celsius
        Celsius = "c",
        /// Fahrenheit
        Fahrenheit = "f",
        /// Kelvin
        Kelvin = "k",
    }
}

/// What every converter prints; part of the tool descriptions (a macro
/// so that `concat!` can splice it into each `long_about`).
macro_rules! output_note {
    () => {
        "Prints one line, `<value> <from> = <result> <to>`, for example `5 km = 3.11 mi`."
    };
}

kanna::cli! {
    /// Convert between units of length, mass and temperature
    #[name = "units", version = env!("CARGO_PKG_VERSION")]
    #[example = "units length 5 --from km --to mi"]
    #[example = "units temp -f c -t f -- -40"]
    #[example = "units --precision 0 mass 3 --from lb --to g"]
    #[example = "units tools --mcp"]
    #[example = "units call units_temp '{\"value\": 100, \"from\": \"c\", \"to\": \"f\"}'"]
    #[derive(Debug)]
    struct Args {
        /// Decimal places in the result
        #[short, global, default = 2]
        precision: usize,
        #[subcommand]
        cmd: Cmd,
    }

    #[derive(Debug)]
    enum Cmd {
        /// Convert a length
        Length(LengthArgs),
        /// Convert a mass
        Mass(MassArgs),
        /// Convert a temperature
        Temp(TempArgs),
        /// Print the tool definitions for an AI agent (Claude by default)
        #[no_tool]
        Tools(ToolsArgs),
        /// Run a tool from its JSON input, as an agent would
        #[no_tool]
        Call(CallArgs),
        /// Print the JSON description of this command
        #[no_tool]
        Schema,
    }

    #[derive(Debug)]
    #[long_about = concat!("Convert a length. ", output_note!())]
    struct LengthArgs {
        /// The quantity to convert
        #[positional]
        value: f64,
        /// Unit of the input
        #[short, value_enum]
        from: Length,
        /// Unit of the result
        #[short, value_enum]
        to: Length,
    }

    #[derive(Debug)]
    #[long_about = concat!("Convert a mass. ", output_note!())]
    struct MassArgs {
        /// The quantity to convert
        #[positional]
        value: f64,
        /// Unit of the input
        #[short, value_enum]
        from: Mass,
        /// Unit of the result
        #[short, value_enum]
        to: Mass,
    }

    #[derive(Debug)]
    #[long_about = concat!("Convert a temperature. ", output_note!())]
    struct TempArgs {
        /// The quantity to convert
        #[positional]
        value: f64,
        /// Unit of the input
        #[short, value_enum]
        from: Temperature,
        /// Unit of the result
        #[short, value_enum]
        to: Temperature,
    }

    #[derive(Debug)]
    struct ToolsArgs {
        /// Use the Model Context Protocol spelling (`inputSchema`)
        mcp: bool,
    }

    #[derive(Debug)]
    struct CallArgs {
        /// Print the argument vector instead of running it
        #[short = 'n']
        dry_run: bool,
        /// Tool name, as printed by `tools`
        #[positional]
        tool: String,
        /// The tool input, a JSON object
        #[positional, value_name = "JSON"]
        input: String,
    }
}

impl Length {
    /// Metres per unit.
    fn factor(self) -> f64 {
        match self {
            Length::Millimetre => 0.001,
            Length::Centimetre => 0.01,
            Length::Metre => 1.0,
            Length::Kilometre => 1000.0,
            Length::Inch => 0.0254,
            Length::Foot => 0.3048,
            Length::Mile => 1609.344,
        }
    }
}

impl Mass {
    /// Grams per unit.
    fn factor(self) -> f64 {
        match self {
            Mass::Gram => 1.0,
            Mass::Kilogram => 1000.0,
            Mass::Ounce => 28.349_523_125,
            Mass::Pound => 453.592_37,
        }
    }
}

impl Temperature {
    fn kelvin(self, v: f64) -> f64 {
        match self {
            Temperature::Celsius => v + 273.15,
            Temperature::Fahrenheit => (v - 32.0) * 5.0 / 9.0 + 273.15,
            Temperature::Kelvin => v,
        }
    }

    fn of_kelvin(self, k: f64) -> f64 {
        match self {
            Temperature::Celsius => k - 273.15,
            Temperature::Fahrenheit => (k - 273.15) * 9.0 / 5.0 + 32.0,
            Temperature::Kelvin => k,
        }
    }
}

/// The converted quantity, formatted with `precision` decimals.
fn convert(args: &Args) -> Option<String> {
    let (value, from, to, result) = match &args.cmd {
        Cmd::Length(a) => (
            a.value,
            a.from.name(),
            a.to.name(),
            a.value * a.from.factor() / a.to.factor(),
        ),
        Cmd::Mass(a) => (
            a.value,
            a.from.name(),
            a.to.name(),
            a.value * a.from.factor() / a.to.factor(),
        ),
        Cmd::Temp(a) => (
            a.value,
            a.from.name(),
            a.to.name(),
            a.to.of_kelvin(a.from.kelvin(a.value)),
        ),
        _ => return None,
    };
    let p = args.precision;
    Some(format!("{value} {from} = {result:.p$} {to}"))
}

fn run(args: Args) -> Result<(), Error> {
    if let Some(line) = convert(&args) {
        println!("{line}");
        return Ok(());
    }
    let cmd = Args::command();
    match args.cmd {
        Cmd::Tools(t) => println!("{}", tool::to_json(&tool::tools(&cmd), t.mcp)),
        Cmd::Schema => print!("{}", kanna_schema::to_json_pretty(&cmd)),
        Cmd::Call(c) => {
            // JSON → argv, checked by parsing; then parse again into the
            // typed `Args` and run it like a command line. The error keeps
            // the parser's kind and argument, so a driver reading
            // KANNA_ERROR_FORMAT=json output can branch on them.
            let argv: Vec<OsString> = tool::to_argv(&cmd, &c.tool, &c.input)
                .map_err(|e| e.with_tip("run 'units tools' to see the tools"))?;
            if c.dry_run {
                let words: Vec<String> = argv
                    .iter()
                    .map(|w| w.to_string_lossy().into_owned())
                    .collect();
                println!("{}", words.join(" "));
            } else {
                run(Args::try_parse_args(argv)?)?;
            }
        }
        Cmd::Length(_) | Cmd::Mass(_) | Cmd::Temp(_) => unreachable!("handled by convert"),
    }
    Ok(())
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        e.exit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanna::ErrorKind;

    #[test]
    fn definition_is_sound() {
        let cmd = Args::command();
        cmd.validate().unwrap();
        cmd.check_examples().unwrap();
    }

    #[test]
    fn converts() {
        let a = Args::try_parse_args(["length", "1", "--from", "mi", "--to", "m"]).unwrap();
        assert_eq!(convert(&a).unwrap(), "1 mi = 1609.34 m");
        let a =
            Args::try_parse_args(["-p", "0", "temp", "-f", "c", "-t", "f", "--", "-40"]).unwrap();
        assert_eq!(convert(&a).unwrap(), "-40 c = -40 f");
        let a = Args::try_parse_args(["mass", "1", "--from", "lb", "--to", "oz"]).unwrap();
        assert_eq!(convert(&a).unwrap(), "1 lb = 16.00 oz");
    }

    #[test]
    fn rejects_bad_input() {
        let e = Args::try_parse_args(["temp", "100", "-f", "c", "-t", "miles"]).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::InvalidValue);
        let e = Args::try_parse_args(["length", "5", "--from", "km"]).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::MissingRequired);
    }

    #[test]
    fn only_the_converters_are_tools() {
        let tools = tool::tools(&Args::command());
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["units_length", "units_mass", "units_temp"]);
        assert_eq!(
            tools[0].description,
            concat!("Convert a length. ", output_note!())
        );
        // Still there for people.
        let help = Args::command().render_help();
        assert!(
            help.contains("tools   Print the tool definitions"),
            "{help}"
        );
    }

    #[test]
    fn tool_schema_uses_ids_types_enums_and_value_help() {
        let tools = tool::tools(&Args::command());
        let temp = tools.iter().find(|t| t.name == "units_temp").unwrap();
        let schema = &temp.input_schema;
        assert!(schema.contains(r#""value":{"type":"number""#), "{schema}");
        assert!(
            schema.contains(
                r#""from":{"type":"string","enum":["c","f","k"],"description":"Unit of the input [possible values: c = Celsius, f = Fahrenheit, k = Kelvin]"}"#
            ),
            "{schema}"
        );
        assert!(
            schema.contains(r#""precision":{"type":"integer","minimum":0"#),
            "{schema}"
        );
        assert!(
            schema.contains(r#""required":["value","from","to"]"#),
            "{schema}"
        );
    }

    #[test]
    fn agent_round_trip() {
        let cmd = Args::command();
        let argv = tool::to_argv(
            &cmd,
            "units_temp",
            r#"{"value": -40, "from": "c", "to": "f", "precision": 1}"#,
        )
        .unwrap();
        assert_eq!(
            argv,
            ["temp", "--from=c", "--to=f", "--precision=1", "--", "-40"]
        );
        let a = Args::try_parse_args(argv).unwrap();
        assert_eq!(convert(&a).unwrap(), "-40 c = -40.0 f");
    }

    #[test]
    fn tool_errors_keep_kind_and_argument() {
        let cmd = Args::command();
        let e = tool::to_argv(
            &cmd,
            "units_temp",
            r#"{"value": "hot", "from": "c", "to": "f"}"#,
        )
        .unwrap_err();
        assert_eq!(
            (e.kind(), e.arg()),
            (ErrorKind::InvalidValue, Some("value"))
        );
        let e = tool::to_argv(&cmd, "units_temp", r#"{"value": 1, "from": "c"}"#).unwrap_err();
        assert_eq!(
            (e.kind(), e.arg()),
            (ErrorKind::MissingRequired, Some("to"))
        );
        let e = tool::to_argv(
            &cmd,
            "units_temp",
            r#"{"value": 1, "from": "c", "to": "f", "unit": "x"}"#,
        )
        .unwrap_err();
        assert_eq!(
            (e.kind(), e.arg()),
            (ErrorKind::UnexpectedArgument, Some("unit"))
        );
        // `no_tool` commands are not callable as tools.
        let e = tool::to_argv(&cmd, "units_call", "{}").unwrap_err();
        assert_eq!(e.message(), "unknown tool 'units_call'");
    }
}
