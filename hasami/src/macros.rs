//! The [`cli!`] declarative DSL, implemented with `macro_rules!` only.

/// Define a command line interface from a struct-like description, without
/// a proc-macro.
///
/// ```
/// hasami::cli! {
///     /// Greet someone
///     #[derive(Debug)]
///     #[name = "greet", version = "1.0"]
///     struct Args {
///         /// Name of the person
///         #[short = 'n'] name: String,
///         /// Number of times
///         #[short, default = 1] count: u8,
///         /// Use upper case
///         shout: bool,
///         #[subcommand] cmd: Option<Cmd>,
///     }
///
///     #[derive(Debug)]
///     enum Cmd {
///         /// Add a thing
///         Add(AddArgs),
///         /// Remove a thing
///         #[name = "rm", alias = "remove"]
///         Remove,
///     }
///
///     #[derive(Debug)]
///     struct AddArgs {
///         /// What to add
///         #[positional] thing: String,
///     }
/// }
///
/// use hasami::Cli;
/// let args = Args::try_parse_args(["-n", "bob", "add", "cake"]).unwrap();
/// assert_eq!(args.name, "bob");
/// assert_eq!(args.count, 1);
/// assert!(matches!(args.cmd, Some(Cmd::Add(AddArgs { ref thing })) if thing == "cake"));
/// ```
///
/// # Grammar
///
/// The macro takes any number of `struct` and `enum` items.
///
/// **Structs** become argument sets. Doc comments, `#[derive(...)]` and
/// hasami settings may appear in any order. Settings are comma-separated
/// inside one or more `#[...]`:
/// `name = expr`, `version = expr`, `long_version = expr`, `about = expr`,
/// `long_about = expr`, `before_help = expr`, `after_help = expr`,
/// `disable_help`, `disable_version`, `args_override_self`,
/// `arg_required_else_help`, `infer_long_args`, `infer_subcommands`,
/// `allow_external_subcommands`, `term_width = expr`.
/// The doc comment is the `about` text. The command name defaults to
/// `CARGO_PKG_NAME`.
///
/// **Fields** map by type:
///
/// | Field type   | Meaning                                   |
/// |--------------|-------------------------------------------|
/// | `bool`       | flag                                      |
/// | `usize`      | value, or a counter with `count`          |
/// | `Option<T>`  | optional value                            |
/// | `Vec<T>`     | repeatable value                          |
/// | `T`          | required value, or `default = expr`       |
///
/// where `T: FromStr + Clone + Send + Sync + 'static` with a `Display`
/// error. Field settings: `long`, `long = "name"`, `short`, `short = 'c'`,
/// `alias = "name"`, `short_alias = 'c'`, `positional`, `count`, `hidden`,
/// `global`, `required` (for `Vec`), `value_name = "NAME"`,
/// `default = expr`, `default_missing = expr`, `env = "VAR"`,
/// `possible = ["a", "b"]`, `value_enum` (possible values from a
/// [`ValueEnum`](crate::ValueEnum) type), `help = "text"`,
/// `long_help = "text"`, `help_heading = "Title"`, `visible_alias = "name"`,
/// `last_wins`, `greedy` and `delimiter = ','` (for `Vec`), `trailing`
/// (for a positional `Vec`), `requires = "field"`,
/// `conflicts_with = "field"`, `required_unless = "field"`,
/// `required_if_eq = ["field", "value"]`, `requires_if = ["value", "field"]`.
/// The long name defaults to the kebab-case field name; the doc comment is
/// the help text.
/// `#[subcommand] field: Option<Enum>` or `#[subcommand] field: Enum`
/// (required) attaches an enum defined with this macro.
/// `#[flatten] field: Struct` inlines every argument, group and constraint
/// of another struct defined with this macro (or `#[derive(Args)]`), so a
/// set of common options can be shared between commands.
///
/// **Enums** become subcommand sets. Each variant is `Name` (no arguments)
/// or `Name(Struct)` where `Struct` is defined with this macro. Variant
/// settings: `name = "x"` (default: kebab-case of the variant),
/// `alias = "x"`, `visible_alias = "x"`, `hidden`. The doc comment is the
/// summary; without one the
/// payload struct's doc comment is used.
///
/// # Generated API
///
/// Structs implement [`Cli`](crate::Cli) (`command`, `from_matches`,
/// `ABOUT`, plus the provided `parse`, `try_parse`, `parse_from`,
/// `try_parse_from` and `try_parse_args`); enums implement
/// [`Subcommands`](crate::Subcommands). Bring the trait into scope to call
/// the methods: `use hasami::Cli;`.
#[macro_export]
macro_rules! cli {
    () => {};
    (
        $(#[$($attr:tt)*])*
        $vis:vis struct $Name:ident { $($body:tt)* }
        $($rest:tt)*
    ) => {
        $crate::__cli_attrs! {
            @struct [$vis] [$Name] { [] [] [] } [ $([$($attr)*])* ] [ $($body)* ] [ $($rest)* ]
        }
    };
    (
        $(#[$($attr:tt)*])*
        $vis:vis enum $Name:ident { $($body:tt)* }
        $($rest:tt)*
    ) => {
        $crate::__cli_attrs! {
            @enum [$vis] [$Name] { [] [] [] } [ $([$($attr)*])* ] [ $($body)* ] [ $($rest)* ]
        }
    };
}

/// Sort item attributes into doc comments, derives and hasami settings.
#[doc(hidden)]
#[macro_export]
macro_rules! __cli_attrs {
    (@$kind:ident $vis:tt $Name:tt { [$($doc:tt)*] $derives:tt $settings:tt }
        [ [doc = $lit:literal] $($more:tt)* ] $body:tt $rest:tt) => {
        $crate::__cli_attrs! { @$kind $vis $Name { [$($doc)* $lit] $derives $settings } [ $($more)* ] $body $rest }
    };
    (@$kind:ident $vis:tt $Name:tt { $doc:tt [$($derives:tt)*] $settings:tt }
        [ [derive($($d:tt)*)] $($more:tt)* ] $body:tt $rest:tt) => {
        $crate::__cli_attrs! { @$kind $vis $Name { $doc [$($derives)* [$($d)*]] $settings } [ $($more)* ] $body $rest }
    };
    (@$kind:ident $vis:tt $Name:tt { $doc:tt $derives:tt [$($settings:tt)*] }
        [ [$($s:tt)*] $($more:tt)* ] $body:tt $rest:tt) => {
        $crate::__cli_attrs! { @$kind $vis $Name { $doc $derives [$($settings)* [$($s)*]] } [ $($more)* ] $body $rest }
    };
    (@struct $vis:tt $Name:tt { $doc:tt $derives:tt $settings:tt } [ ] [ $($body:tt)* ] [ $($rest:tt)* ]) => {
        $crate::__cli_fields! { @munch [ $vis $Name $doc $derives $settings ] { } [ $($body)* ] }
        $crate::cli! { $($rest)* }
    };
    (@enum $vis:tt $Name:tt { $doc:tt $derives:tt $settings:tt } [ ] [ $($body:tt)* ] [ $($rest:tt)* ]) => {
        $crate::__cli_enum! { $vis $Name $doc $derives [ $($body)* ] }
        $crate::cli! { $($rest)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_fields {
    (@munch $ctx:tt { $($done:tt)* } [ ]) => {
        $crate::__cli_emit! { $ctx { $($done)* } }
    };
    (@munch $ctx:tt { $($done:tt)* }
        [ $(#[doc = $d:literal])* #[flatten] $fvis:vis $fname:ident : $t:ty $(, $($rest:tt)*)? ]) => {
        $crate::__cli_fields! { @munch $ctx { $($done)* [ $fvis $fname [flatten $t] [] ] } [ $($($rest)*)? ] }
    };
    (@munch $ctx:tt { $($done:tt)* }
        [ $(#[doc = $d:literal])* #[subcommand] $fvis:vis $fname:ident : Option<$t:ty> $(, $($rest:tt)*)? ]) => {
        $crate::__cli_fields! { @munch $ctx { $($done)* [ $fvis $fname [subopt $t] [] ] } [ $($($rest)*)? ] }
    };
    (@munch $ctx:tt { $($done:tt)* }
        [ $(#[doc = $d:literal])* #[subcommand] $fvis:vis $fname:ident : $t:ty $(, $($rest:tt)*)? ]) => {
        $crate::__cli_fields! { @munch $ctx { $($done)* [ $fvis $fname [subreq $t] [] ] } [ $($($rest)*)? ] }
    };
    (@munch $ctx:tt { $($done:tt)* }
        [ $(#[$($fa:tt)*])* $fvis:vis $fname:ident : Option<$t:ty> $(, $($rest:tt)*)? ]) => {
        $crate::__cli_fields! { @munch $ctx { $($done)* [ $fvis $fname [option $t] [$([$($fa)*])*] ] } [ $($($rest)*)? ] }
    };
    (@munch $ctx:tt { $($done:tt)* }
        [ $(#[$($fa:tt)*])* $fvis:vis $fname:ident : Vec<$t:ty> $(, $($rest:tt)*)? ]) => {
        $crate::__cli_fields! { @munch $ctx { $($done)* [ $fvis $fname [vec $t] [$([$($fa)*])*] ] } [ $($($rest)*)? ] }
    };
    (@munch $ctx:tt { $($done:tt)* }
        [ $(#[$($fa:tt)*])* $fvis:vis $fname:ident : bool $(, $($rest:tt)*)? ]) => {
        $crate::__cli_fields! { @munch $ctx { $($done)* [ $fvis $fname [flag] [$([$($fa)*])*] ] } [ $($($rest)*)? ] }
    };
    (@munch $ctx:tt { $($done:tt)* }
        [ $(#[$($fa:tt)*])* $fvis:vis $fname:ident : usize $(, $($rest:tt)*)? ]) => {
        $crate::__cli_fields! { @munch $ctx { $($done)* [ $fvis $fname [usize] [$([$($fa)*])*] ] } [ $($($rest)*)? ] }
    };
    (@munch $ctx:tt { $($done:tt)* }
        [ $(#[$($fa:tt)*])* $fvis:vis $fname:ident : $t:ty $(, $($rest:tt)*)? ]) => {
        $crate::__cli_fields! { @munch $ctx { $($done)* [ $fvis $fname [plain $t] [$([$($fa)*])*] ] } [ $($($rest)*)? ] }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_emit {
    (
        [ [$vis:vis] [$Name:ident] [$($doc:literal)*] [$([$($derive:tt)*])*] [$([$($setting:tt)*])*] ]
        { $( [ $fvis:vis $fname:ident [$($kind:tt)*] [$([$($fa:tt)*])*] ] )* }
    ) => {
        $(#[doc = $doc])*
        $(#[derive($($derive)*)])*
        $vis struct $Name {
            $( $fvis $fname : $crate::__cli_field_type!($($kind)*), )*
        }

        #[allow(unused_variables, clippy::all)]
        impl $Name {
            #[doc(hidden)]
            pub fn __spec() -> ( $( $crate::__cli_arg_type!($($kind)*), )* ) {
                ( $( $crate::__cli_build_arg!($fname [$($kind)*] [$([$($fa)*])*]), )* )
            }
        }

        #[allow(unused_variables, clippy::all)]
        impl $crate::Cli for $Name {
            const ABOUT: Option<&'static str> = $crate::__cli_about!($($doc)*);

            fn command() -> $crate::Command {
                let ( $($fname,)* ) = Self::__spec();
                let mut __spec = $crate::__macro::CommandSpec::new(env!("CARGO_PKG_NAME"));
                $( __spec.doc($doc); )*
                $( $crate::__cli_cmd_settings!(__spec [$($setting)*]); )*
                let mut __cmd = __spec.finish();
                $( __cmd = $crate::__cli_add_arg!(__cmd, $fname, [$($kind)*]); )*
                __cmd
            }

            fn from_matches(__m: &$crate::Matches) -> Result<Self, $crate::Error> {
                let ( $($fname,)* ) = Self::__spec();
                Ok(Self { $( $fname: $crate::__cli_extract!(__m, $fname, [$($kind)*]), )* })
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_about {
    () => { None };
    ($first:literal $($rest:literal)*) => { Some(concat!($first $(, "\n", $rest)*)) };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_field_type {
    (option $t:ty) => { Option<$t> };
    (vec $t:ty) => { Vec<$t> };
    (flag) => { bool };
    (usize) => { usize };
    (plain $t:ty) => { $t };
    (subopt $t:ty) => { Option<$t> };
    (subreq $t:ty) => { $t };
    (flatten $t:ty) => { $t };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_arg_type {
    (option $t:ty) => { $crate::Arg<Option<$t>> };
    (vec $t:ty) => { $crate::Arg<Vec<$t>> };
    (flag) => { $crate::Arg<bool> };
    (usize) => { $crate::Arg<usize> };
    (plain $t:ty) => { $crate::Arg<$t> };
    (subopt $t:ty) => { () };
    (subreq $t:ty) => { () };
    (flatten $t:ty) => { () };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_build_arg {
    ($fname:ident [option $t:ty] [$([$($fa:tt)*])*]) => {{
        let mut __s = $crate::__macro::FieldSpec::<$t>::new(stringify!($fname));
        $( $crate::__cli_field_settings!(__s [$($fa)*]); )*
        __s.option()
    }};
    ($fname:ident [vec $t:ty] [$([$($fa:tt)*])*]) => {{
        let mut __s = $crate::__macro::FieldSpec::<$t>::new(stringify!($fname));
        $( $crate::__cli_field_settings!(__s [$($fa)*]); )*
        __s.vec()
    }};
    ($fname:ident [flag] [$([$($fa:tt)*])*]) => {{
        let mut __s = $crate::__macro::FieldSpec::<bool>::new(stringify!($fname));
        $( $crate::__cli_field_settings!(__s [$($fa)*]); )*
        __s.flag()
    }};
    ($fname:ident [usize] [$([$($fa:tt)*])*]) => {{
        let mut __s = $crate::__macro::FieldSpec::<usize>::new(stringify!($fname));
        $( $crate::__cli_field_settings!(__s [$($fa)*]); )*
        __s.usize_field()
    }};
    ($fname:ident [plain $t:ty] [$([$($fa:tt)*])*]) => {{
        let mut __s = $crate::__macro::FieldSpec::<$t>::new(stringify!($fname));
        $( $crate::__cli_field_settings!(__s [$($fa)*]); )*
        __s.plain()
    }};
    ($fname:ident [subopt $t:ty] []) => { () };
    ($fname:ident [subreq $t:ty] []) => { () };
    ($fname:ident [flatten $t:ty] []) => { () };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_field_settings {
    ($s:ident []) => {};
    ($s:ident [doc = $d:literal $(, $($rest:tt)*)?]) => {
        $s.doc($d); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [help = $v:literal $(, $($rest:tt)*)?]) => {
        $s.help($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [long = $v:literal $(, $($rest:tt)*)?]) => {
        $s.long($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [long $(, $($rest:tt)*)?]) => {
        $s.long_auto(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [short = $v:literal $(, $($rest:tt)*)?]) => {
        $s.short($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [short $(, $($rest:tt)*)?]) => {
        $s.short_auto(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [alias = $v:literal $(, $($rest:tt)*)?]) => {
        $s.alias($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [short_alias = $v:literal $(, $($rest:tt)*)?]) => {
        $s.short_alias($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [hidden $(, $($rest:tt)*)?]) => {
        $s.hidden(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [global $(, $($rest:tt)*)?]) => {
        $s.global(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [positional $(, $($rest:tt)*)?]) => {
        $s.positional(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [count $(, $($rest:tt)*)?]) => {
        $s.count(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [required $(, $($rest:tt)*)?]) => {
        $s.required(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [value_name = $v:literal $(, $($rest:tt)*)?]) => {
        $s.value_name($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [default = $v:expr $(, $($rest:tt)*)?]) => {
        $s.default($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [default_missing = $v:expr $(, $($rest:tt)*)?]) => {
        $s.default_missing($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [env = $v:literal $(, $($rest:tt)*)?]) => {
        $s.env($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [possible = [$($p:literal),* $(,)?] $(, $($rest:tt)*)?]) => {
        $s.possible(&[$($p),*]); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [value_enum $(, $($rest:tt)*)?]) => {
        $s.value_enum(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [visible_alias = $v:literal $(, $($rest:tt)*)?]) => {
        $s.visible_alias($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [long_help = $v:literal $(, $($rest:tt)*)?]) => {
        $s.long_help($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [help_heading = $v:literal $(, $($rest:tt)*)?]) => {
        $s.help_heading($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [last_wins $(, $($rest:tt)*)?]) => {
        $s.last_wins(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [greedy $(, $($rest:tt)*)?]) => {
        $s.greedy(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [trailing $(, $($rest:tt)*)?]) => {
        $s.trailing(); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [delimiter = $v:literal $(, $($rest:tt)*)?]) => {
        $s.delimiter($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [requires = $v:literal $(, $($rest:tt)*)?]) => {
        $s.requires($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [conflicts_with = $v:literal $(, $($rest:tt)*)?]) => {
        $s.conflicts_with($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [required_unless = $v:literal $(, $($rest:tt)*)?]) => {
        $s.required_unless($v); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [required_if_eq = [$a:literal, $b:literal] $(, $($rest:tt)*)?]) => {
        $s.required_if_eq(&[$a, $b]); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [requires_if = [$a:literal, $b:literal] $(, $($rest:tt)*)?]) => {
        $s.requires_if(&[$a, $b]); $crate::__cli_field_settings!($s [$($($rest)*)?]);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_cmd_settings {
    ($s:ident []) => {};
    ($s:ident [doc = $d:literal $(, $($rest:tt)*)?]) => {
        $s.doc($d); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [name = $v:expr $(, $($rest:tt)*)?]) => {
        $s.name($v); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [version = $v:expr $(, $($rest:tt)*)?]) => {
        $s.version($v); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [about = $v:expr $(, $($rest:tt)*)?]) => {
        $s.about($v); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [long_about = $v:expr $(, $($rest:tt)*)?]) => {
        $s.long_about($v); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [after_help = $v:expr $(, $($rest:tt)*)?]) => {
        $s.after_help($v); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [disable_help $(, $($rest:tt)*)?]) => {
        $s.disable_help(); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [disable_version $(, $($rest:tt)*)?]) => {
        $s.disable_version(); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [before_help = $v:expr $(, $($rest:tt)*)?]) => {
        $s.before_help($v); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [long_version = $v:expr $(, $($rest:tt)*)?]) => {
        $s.long_version($v); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [args_override_self $(, $($rest:tt)*)?]) => {
        $s.args_override_self(); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [arg_required_else_help $(, $($rest:tt)*)?]) => {
        $s.arg_required_else_help(); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [infer_long_args $(, $($rest:tt)*)?]) => {
        $s.infer_long_args(); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [infer_subcommands $(, $($rest:tt)*)?]) => {
        $s.infer_subcommands(); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [allow_external_subcommands $(, $($rest:tt)*)?]) => {
        $s.allow_external_subcommands(); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [term_width = $v:expr $(, $($rest:tt)*)?]) => {
        $s.term_width($v); $crate::__cli_cmd_settings!($s [$($($rest)*)?]);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_add_arg {
    ($cmd:ident, $fname:ident, [subopt $t:ty]) => {
        $crate::__macro::add_subcommands($cmd, <$t as $crate::Subcommands>::subcommands(), false)
    };
    ($cmd:ident, $fname:ident, [subreq $t:ty]) => {
        $crate::__macro::add_subcommands($cmd, <$t as $crate::Subcommands>::subcommands(), true)
    };
    ($cmd:ident, $fname:ident, [flatten $t:ty]) => {
        $crate::__macro::flatten($cmd, <$t as $crate::Cli>::command())
    };
    ($cmd:ident, $fname:ident, [$($kind:tt)*]) => {
        $cmd.arg(&$fname)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_extract {
    ($m:ident, $fname:ident, [subopt $t:ty]) => {
        <$t as $crate::Subcommands>::from_matches($m)?
    };
    ($m:ident, $fname:ident, [subreq $t:ty]) => {
        $crate::__macro::required_subcommand(<$t as $crate::Subcommands>::from_matches($m)?)?
    };
    ($m:ident, $fname:ident, [flatten $t:ty]) => {
        <$t as $crate::Cli>::from_matches($m)?
    };
    ($m:ident, $fname:ident, [$($kind:tt)*]) => {
        $m.get(&$fname)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_enum {
    (
        [$vis:vis] [$Name:ident] [$($doc:literal)*] [$([$($derive:tt)*])*]
        [ $( $(#[$($va:tt)*])* $Variant:ident $( ( $Payload:ty ) )? ),* $(,)? ]
    ) => {
        $(#[doc = $doc])*
        $(#[derive($($derive)*)])*
        $vis enum $Name {
            $( $Variant $( ( $Payload ) )? ),*
        }

        #[allow(unused_variables, clippy::all)]
        impl $crate::Subcommands for $Name {
            fn subcommands() -> Vec<$crate::Subcommand> {
                vec![
                    $( {
                        let mut __s = $crate::__macro::SubSpec::new(stringify!($Variant));
                        $( $crate::__cli_sub_settings!(__s [$($va)*]); )*
                        $crate::__cli_sub_finish!(__s $( ( $Payload ) )?)
                    } ),*
                ]
            }

            fn from_matches(__m: &$crate::Matches) -> Result<Option<Self>, $crate::Error> {
                let Some((__name, __sub)) = __m.subcommand() else {
                    return Ok(None);
                };
                $( {
                    let mut __s = $crate::__macro::SubSpec::new(stringify!($Variant));
                    $( $crate::__cli_sub_settings!(__s [$($va)*]); )*
                    if __s.subcommand_name() == __name {
                        return Ok(Some($crate::__cli_sub_construct!(Self::$Variant, __sub $( ( $Payload ) )?)));
                    }
                } )*
                Ok(None)
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_sub_settings {
    ($s:ident []) => {};
    ($s:ident [doc = $d:literal $(, $($rest:tt)*)?]) => {
        $s.doc($d); $crate::__cli_sub_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [name = $v:literal $(, $($rest:tt)*)?]) => {
        $s.name($v); $crate::__cli_sub_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [alias = $v:literal $(, $($rest:tt)*)?]) => {
        $s.alias($v); $crate::__cli_sub_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [hidden $(, $($rest:tt)*)?]) => {
        $s.hidden(); $crate::__cli_sub_settings!($s [$($($rest)*)?]);
    };
    ($s:ident [visible_alias = $v:literal $(, $($rest:tt)*)?]) => {
        $s.visible_alias($v); $crate::__cli_sub_settings!($s [$($($rest)*)?]);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_sub_finish {
    ($s:ident) => {
        $s.finish(None, $crate::__macro::unit_command)
    };
    ($s:ident ($Payload:ty)) => {
        $s.finish(<$Payload>::ABOUT, <$Payload>::command)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cli_sub_construct {
    ($variant:path, $sub:ident) => {
        $variant
    };
    ($variant:path, $sub:ident ($Payload:ty)) => {
        $variant(<$Payload>::from_matches($sub)?)
    };
}
