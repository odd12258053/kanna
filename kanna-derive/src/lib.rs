//! `#[derive(Args)]`, `#[derive(Commands)]` and `#[derive(ValueEnum)]` for
//! [kanna](https://crates.io/crates/kanna).
//!
//! Enable with kanna's `derive` feature and use through `kanna::Args` /
//! `kanna::Commands`; this crate is not meant to be depended on directly.
//!
//! The derives generate exactly the code the `kanna::cli!` macro would
//! generate for the same definition: both lower to the same runtime
//! `FieldSpec`/`CommandSpec`/`SubSpec` calls, so help, errors and matches
//! cannot differ between the two front ends.
//!
//! ```ignore
//! use kanna::{Args, Cli, Commands};
//!
//! /// Greet someone
//! #[derive(Args)]
//! #[kanna(name = "greet", version = "1.0")]
//! struct Greet {
//!     /// Name of the person
//!     #[kanna(short = 'n')]
//!     name: String,
//!     /// Number of times
//!     #[kanna(short, default = 1)]
//!     count: u8,
//!     #[kanna(subcommand)]
//!     cmd: Option<Cmd>,
//! }
//!
//! #[derive(Commands)]
//! enum Cmd {
//!     /// Add a thing
//!     Add(AddArgs),
//!     /// Remove a thing
//!     #[kanna(name = "rm")]
//!     Remove,
//! }
//!
//! #[derive(Args)]
//! struct AddArgs {
//!     #[kanna(positional)]
//!     thing: String,
//! }
//! ```
//!
//! The settings vocabulary is the same as `cli!`'s, written inside
//! `#[kanna(...)]`; see the `kanna::cli!` documentation.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::TokenTree;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Error, Expr, Fields, GenericArgument, Ident, LitStr,
    PathArguments, Result, Token, Type,
};

/// Derive `kanna::Cli` for a struct with named fields.
#[proc_macro_derive(Args, attributes(kanna))]
pub fn derive_args(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    expand_args(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Derive `kanna::Subcommands` for an enum whose variants are unit-like or
/// carry one `Cli` struct.
#[proc_macro_derive(Commands, attributes(kanna))]
pub fn derive_commands(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    expand_commands(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Derive `kanna::ValueEnum`, `FromStr` and `Display` for a fieldless
/// enum. Each variant is written in kebab-case on the command line unless
/// `#[kanna(name = "x")]` says otherwise; the first line of a variant's
/// doc comment is its help. The enum must be `Clone`.
#[proc_macro_derive(ValueEnum, attributes(kanna))]
pub fn derive_value_enum(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    expand_value_enum(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_value_enum(input: &DeriveInput) -> Result<TokenStream2> {
    let variants = match &input.data {
        Data::Enum(e) => &e.variants,
        _ => {
            return Err(Error::new(
                input.ident.span(),
                "#[derive(ValueEnum)] only supports enums",
            ));
        }
    };
    let name = &input.ident;
    let mut idents = Vec::new();
    let mut names = Vec::new();
    let mut helps = Vec::new();
    for v in variants {
        if !matches!(v.fields, Fields::Unit) {
            return Err(Error::new(
                v.fields.span(),
                "#[derive(ValueEnum)] variants cannot have fields",
            ));
        }
        let attrs = collect_attrs(&v.attrs)?;
        let mut text = kebab_case(&v.ident.to_string());
        for s in &attrs.settings {
            match (s.key.to_string().as_str(), &s.value) {
                ("name", Some(v)) => {
                    let lit: LitStr = syn::parse2(v.clone())?;
                    text = lit.value();
                }
                _ => {
                    return Err(Error::new(
                        s.key.span(),
                        "the only setting on a ValueEnum variant is `name = \"..\"`",
                    ));
                }
            }
        }
        idents.push(v.ident.clone());
        names.push(text);
        // The first doc line is the help, as in `value_enum!`.
        let doc = attrs.docs.first().map(|d| d.value()).unwrap_or_default();
        let doc = doc.trim();
        helps.push(if doc.is_empty() {
            quote! { None }
        } else {
            quote! { Some(#doc) }
        });
    }
    Ok(quote! {
        impl ::kanna::ValueEnum for #name {
            const VALUES: &'static [#name] = &[ #( #name::#idents ),* ];

            fn name(&self) -> &'static str {
                match self {
                    #( #name::#idents => #names, )*
                }
            }

            fn help(&self) -> Option<&'static str> {
                match self {
                    #( #name::#idents => #helps, )*
                }
            }
        }

        impl ::std::str::FromStr for #name {
            type Err = String;

            fn from_str(s: &str) -> Result<#name, String> {
                <#name as ::kanna::ValueEnum>::from_name(s).ok_or_else(|| {
                    ::kanna::__macro::unknown_enum_value(s, &<#name as ::kanna::ValueEnum>::names())
                })
            }
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(<#name as ::kanna::ValueEnum>::name(self))
            }
        }
    })
}

/// `CamelCase` to `kebab-case`, as `kanna::__macro::camel_kebab` does at
/// runtime for `cli!`.
fn kebab_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c == '_' {
            out.push('-');
        } else if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// One `key` or `key = value` inside `#[kanna(...)]`. The value is kept as
/// raw tokens (everything up to the next top-level comma) so that any
/// expression is accepted without syn's `full` feature.
struct Setting {
    key: Ident,
    value: Option<TokenStream2>,
}

impl Parse for Setting {
    fn parse(input: ParseStream<'_>) -> Result<Setting> {
        let key: Ident = input.parse()?;
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let mut tokens = TokenStream2::new();
            while !input.is_empty() && !input.peek(Token![,]) {
                let tt: TokenTree = input.parse()?;
                tokens.extend(std::iter::once(tt));
            }
            if tokens.is_empty() {
                return Err(input.error("expected a value after `=`"));
            }
            Some(tokens)
        } else {
            None
        };
        Ok(Setting { key, value })
    }
}

/// Doc comment lines and `#[kanna(...)]` settings of one item.
struct Attrs {
    docs: Vec<LitStr>,
    settings: Vec<Setting>,
}

fn collect_attrs(attrs: &[Attribute]) -> Result<Attrs> {
    let mut out = Attrs {
        docs: Vec::new(),
        settings: Vec::new(),
    };
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    out.docs.push(s.clone());
                }
            }
        } else if attr.path().is_ident("kanna") {
            let list = attr.parse_args_with(Punctuated::<Setting, Token![,]>::parse_terminated)?;
            out.settings.extend(list);
        }
    }
    Ok(out)
}

/// Emit `spec.doc(..)` for every doc line and one call per setting,
/// translating the bare/valued forms exactly like the `cli!` munchers.
fn setting_calls(spec: &Ident, attrs: &Attrs, allowed: &[&str]) -> Result<TokenStream2> {
    let mut calls = TokenStream2::new();
    for d in &attrs.docs {
        calls.extend(quote! { #spec.doc(#d); });
    }
    for s in &attrs.settings {
        let key = s.key.to_string();
        if !allowed.contains(&key.as_str()) {
            return Err(Error::new(
                s.key.span(),
                format!(
                    "unknown kanna setting `{key}`; expected one of: {}",
                    allowed.join(", ")
                ),
            ));
        }
        let call = match (key.as_str(), &s.value) {
            ("long", None) => quote! { #spec.long_auto(); },
            ("short", None) => quote! { #spec.short_auto(); },
            ("possible" | "required_if_eq" | "requires_if", Some(v)) => {
                let method = &s.key;
                quote! { #spec.#method(&#v); }
            }
            (_, Some(v)) => {
                let method = &s.key;
                quote! { #spec.#method(#v); }
            }
            (_, None) => {
                let method = &s.key;
                quote! { #spec.#method(); }
            }
        };
        calls.extend(call);
    }
    Ok(calls)
}

const FIELD_SETTINGS: &[&str] = &[
    "long",
    "short",
    "alias",
    "short_alias",
    "hidden",
    "global",
    "positional",
    "count",
    "required",
    "value_name",
    "default",
    "default_missing",
    "env",
    "possible",
    "value_enum",
    "help",
    "long_help",
    "help_heading",
    "visible_alias",
    "last_wins",
    "greedy",
    "trailing",
    "delimiter",
    "requires",
    "conflicts_with",
    "required_unless",
    "required_if_eq",
    "requires_if",
];

const COMMAND_SETTINGS: &[&str] = &[
    "name",
    "version",
    "long_version",
    "about",
    "long_about",
    "before_help",
    "after_help",
    "disable_help",
    "disable_version",
    "args_override_self",
    "arg_required_else_help",
    "infer_long_args",
    "infer_subcommands",
    "allow_external_subcommands",
    "term_width",
    "example",
];

const VARIANT_SETTINGS: &[&str] = &["name", "alias", "visible_alias", "hidden", "no_tool"];

/// How a field maps onto an argument, mirroring `__cli_fields!`.
enum Kind {
    Flag,
    Usize,
    Option(Type),
    Vec(Type),
    Plain(Type),
    SubOpt(Type),
    SubReq(Type),
    Flatten(Type),
}

fn classify(ty: &Type, subcommand: bool) -> Kind {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            let ident = seg.ident.to_string();
            let inner = || match &seg.arguments {
                PathArguments::AngleBracketed(a) if a.args.len() == 1 => match a.args.first() {
                    Some(GenericArgument::Type(t)) => Some(t.clone()),
                    _ => None,
                },
                _ => None,
            };
            if subcommand {
                return match (ident.as_str(), inner()) {
                    ("Option", Some(t)) => Kind::SubOpt(t),
                    _ => Kind::SubReq(ty.clone()),
                };
            }
            match (ident.as_str(), &seg.arguments) {
                ("bool", PathArguments::None) => return Kind::Flag,
                ("usize", PathArguments::None) => return Kind::Usize,
                ("Option", _) => {
                    if let Some(t) = inner() {
                        return Kind::Option(t);
                    }
                }
                ("Vec", _) => {
                    if let Some(t) = inner() {
                        return Kind::Vec(t);
                    }
                }
                _ => {}
            }
        }
    }
    Kind::Plain(ty.clone())
}

fn expand_args(input: &DeriveInput) -> Result<TokenStream2> {
    if !input.generics.params.is_empty() {
        return Err(Error::new(
            input.generics.span(),
            "#[derive(Args)] does not support generics",
        ));
    }
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            other => {
                return Err(Error::new(
                    other.span(),
                    "#[derive(Args)] needs a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(Error::new(
                input.ident.span(),
                "#[derive(Args)] only supports structs",
            ));
        }
    };
    let name = &input.ident;
    let attrs = collect_attrs(&input.attrs)?;
    let spec = format_ident!("__spec");
    let cmd_settings = setting_calls(&spec, &attrs, COMMAND_SETTINGS)?;
    let about = match attrs.docs.as_slice() {
        [] => quote! { None },
        docs => quote! { Some(concat!(#(#docs),*)) },
    };
    // `concat!` joins without separators; insert newlines between lines.
    let about = if attrs.docs.len() > 1 {
        let mut pieces = Vec::new();
        for (i, d) in attrs.docs.iter().enumerate() {
            if i > 0 {
                pieces.push(quote! { "\n" });
            }
            pieces.push(quote! { #d });
        }
        quote! { Some(concat!(#(#pieces),*)) }
    } else {
        about
    };

    let mut names = Vec::new();
    let mut arg_types = Vec::new();
    let mut builders = Vec::new();
    let mut adds = Vec::new();
    let mut extracts = Vec::new();
    let s = format_ident!("__s");
    let m = format_ident!("__m");
    let cmd = format_ident!("__cmd");
    for f in fields {
        let fname = f
            .ident
            .as_ref()
            .ok_or_else(|| Error::new(f.span(), "named field expected"))?;
        let fattrs = collect_attrs(&f.attrs)?;
        let is_sub = fattrs.settings.iter().any(|s| s.key == "subcommand");
        let is_flat = fattrs.settings.iter().any(|s| s.key == "flatten");
        let kind = if is_flat {
            Kind::Flatten(f.ty.clone())
        } else {
            classify(&f.ty, is_sub)
        };
        let fname_str = fname.to_string();
        let calls = if is_sub || is_flat {
            let others: Vec<&Setting> = fattrs
                .settings
                .iter()
                .filter(|s| s.key != "subcommand" && s.key != "flatten")
                .collect();
            if let Some(extra) = others.first() {
                return Err(Error::new(
                    extra.key.span(),
                    "a #[kanna(subcommand)] or #[kanna(flatten)] field takes no other settings",
                ));
            }
            TokenStream2::new()
        } else {
            setting_calls(&s, &fattrs, FIELD_SETTINGS)?
        };
        let (arg_ty, builder, add, extract) = match &kind {
            Kind::Flag => (
                quote! { ::kanna::Arg<bool> },
                quote! {{ let mut #s = ::kanna::__macro::FieldSpec::<bool>::new(#fname_str); #calls #s.flag() }},
                quote! { #cmd.arg(&#fname) },
                quote! { #m.get(&#fname) },
            ),
            Kind::Usize => (
                quote! { ::kanna::Arg<usize> },
                quote! {{ let mut #s = ::kanna::__macro::FieldSpec::<usize>::new(#fname_str); #calls #s.usize_field() }},
                quote! { #cmd.arg(&#fname) },
                quote! { #m.get(&#fname) },
            ),
            Kind::Option(t) => (
                quote! { ::kanna::Arg<Option<#t>> },
                quote! {{ let mut #s = ::kanna::__macro::FieldSpec::<#t>::new(#fname_str); #calls #s.option() }},
                quote! { #cmd.arg(&#fname) },
                quote! { #m.get(&#fname) },
            ),
            Kind::Vec(t) => (
                quote! { ::kanna::Arg<Vec<#t>> },
                quote! {{ let mut #s = ::kanna::__macro::FieldSpec::<#t>::new(#fname_str); #calls #s.vec() }},
                quote! { #cmd.arg(&#fname) },
                quote! { #m.get(&#fname) },
            ),
            Kind::Plain(t) => (
                quote! { ::kanna::Arg<#t> },
                quote! {{ let mut #s = ::kanna::__macro::FieldSpec::<#t>::new(#fname_str); #calls #s.plain() }},
                quote! { #cmd.arg(&#fname) },
                quote! { #m.get(&#fname) },
            ),
            Kind::SubOpt(t) => (
                quote! { () },
                quote! { () },
                quote! { ::kanna::__macro::add_subcommands(#cmd, <#t as ::kanna::Subcommands>::subcommands(), false) },
                quote! { <#t as ::kanna::Subcommands>::from_matches(#m)? },
            ),
            Kind::SubReq(t) => (
                quote! { () },
                quote! { () },
                quote! { ::kanna::__macro::add_subcommands(#cmd, <#t as ::kanna::Subcommands>::subcommands(), true) },
                quote! { ::kanna::__macro::required_subcommand(<#t as ::kanna::Subcommands>::from_matches(#m)?)? },
            ),
            Kind::Flatten(t) => (
                quote! { () },
                quote! { () },
                quote! { ::kanna::__macro::flatten(#cmd, <#t as ::kanna::Cli>::command()) },
                quote! { <#t as ::kanna::Cli>::from_matches(#m)? },
            ),
        };
        names.push(fname.clone());
        arg_types.push(arg_ty);
        builders.push(builder);
        adds.push(add);
        extracts.push(extract);
    }
    Ok(quote! {
        #[allow(unused_variables, clippy::all)]
        impl #name {
            #[doc(hidden)]
            pub fn __spec() -> ( #(#arg_types,)* ) {
                ( #(#builders,)* )
            }
        }

        #[allow(unused_variables, clippy::all)]
        impl ::kanna::Cli for #name {
            const ABOUT: Option<&'static str> = #about;

            fn command() -> ::kanna::Command {
                let ( #(#names,)* ) = Self::__spec();
                let mut #spec = ::kanna::__macro::CommandSpec::new(env!("CARGO_PKG_NAME"));
                #cmd_settings
                let mut #cmd = #spec.finish();
                #( #cmd = #adds; )*
                #cmd
            }

            fn from_matches(#m: &::kanna::Matches) -> Result<Self, ::kanna::Error> {
                let ( #(#names,)* ) = Self::__spec();
                Ok(Self { #( #names: #extracts, )* })
            }
        }
    })
}

fn expand_commands(input: &DeriveInput) -> Result<TokenStream2> {
    let variants = match &input.data {
        Data::Enum(e) => &e.variants,
        _ => {
            return Err(Error::new(
                input.ident.span(),
                "#[derive(Commands)] only supports enums",
            ));
        }
    };
    let name = &input.ident;
    let s = format_ident!("__s");
    let m = format_ident!("__m");
    let sub = format_ident!("__sub");
    let mut entries = Vec::new();
    let mut arms = Vec::new();
    for v in variants {
        let vname = &v.ident;
        let vname_str = vname.to_string();
        let vattrs = collect_attrs(&v.attrs)?;
        // Variant docs are the summary; setting_calls emits `doc` for them.
        let calls = setting_calls(&s, &vattrs, VARIANT_SETTINGS)?;
        let payload = match &v.fields {
            Fields::Unit => None,
            Fields::Unnamed(u) if u.unnamed.len() == 1 => Some(&u.unnamed[0].ty),
            other => {
                return Err(Error::new(
                    other.span(),
                    "a subcommand variant is either unit-like or carries exactly one Cli struct",
                ));
            }
        };
        let (finish, construct) = match payload {
            None => (
                quote! { #s.finish(None, ::kanna::__macro::unit_command) },
                quote! { Self::#vname },
            ),
            Some(t) => (
                quote! { #s.finish(<#t as ::kanna::Cli>::ABOUT, <#t as ::kanna::Cli>::command) },
                quote! { Self::#vname(<#t as ::kanna::Cli>::from_matches(#sub)?) },
            ),
        };
        entries.push(quote! {{
            let mut #s = ::kanna::__macro::SubSpec::new(#vname_str);
            #calls
            #finish
        }});
        arms.push(quote! {{
            let mut #s = ::kanna::__macro::SubSpec::new(#vname_str);
            #calls
            if #s.subcommand_name() == __name {
                return Ok(Some(#construct));
            }
        }});
    }
    Ok(quote! {
        #[allow(unused_variables, clippy::all)]
        impl ::kanna::Subcommands for #name {
            fn subcommands() -> Vec<::kanna::Subcommand> {
                vec![ #(#entries),* ]
            }

            fn from_matches(#m: &::kanna::Matches) -> Result<Option<Self>, ::kanna::Error> {
                let Some((__name, #sub)) = #m.subcommand() else {
                    return Ok(None);
                };
                #(#arms)*
                Ok(None)
            }
        }
    })
}
