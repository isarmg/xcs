use std::iter::FromIterator;

use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use rust_syntax::spanned::Spanned;
use rust_syntax::{
    Attribute, Expr, Ident, LitStr, Token, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use crate::utils::Sp;

#[derive(Clone)]
pub(crate) struct CommandLineAttr {
    pub(crate) kind: Sp<AttrKind>,
    pub(crate) name: Ident,
    pub(crate) magic: Option<MagicAttrName>,
    pub(crate) value: Option<AttrValue>,
}

impl CommandLineAttr {
    pub(crate) fn parse_all(all_attrs: &[Attribute]) -> Result<Vec<Self>, rust_syntax::Error> {
        let mut parsed = Vec::new();
        for attr in all_attrs {
            let kind = if attr.path().is_ident("command") {
                Sp::new(AttrKind::Command, attr.path().span())
            } else if attr.path().is_ident("group") {
                Sp::new(AttrKind::Group, attr.path().span())
            } else if attr.path().is_ident("arg") {
                Sp::new(AttrKind::Arg, attr.path().span())
            } else if attr.path().is_ident("value") {
                Sp::new(AttrKind::Value, attr.path().span())
            } else {
                continue;
            };
            for mut attr in attr.parse_args_with(Punctuated::<Self, Token![,]>::parse_terminated)? {
                attr.kind = kind;
                parsed.push(attr);
            }
        }
        Ok(parsed)
    }

    pub(crate) fn value_or_abort(&self) -> Result<&AttrValue, rust_syntax::Error> {
        self.value
            .as_ref()
            .ok_or_else(|| format_err!(self.name, "attribute `{}` requires a value", self.name))
    }

    pub(crate) fn lit_str_or_abort(&self) -> Result<&LitStr, rust_syntax::Error> {
        let value = self.value_or_abort()?;
        match value {
            AttrValue::Expr(Expr::Lit(expr)) => match &expr.lit {
                rust_syntax::Lit::Str(lit) => Ok(lit),
                _ => {
                    abort!(
                        expr,
                        "attribute `{}` can only accept string literals",
                        self.name
                    )
                }
            },
            AttrValue::Expr(expr) => {
                abort!(
                    expr,
                    "attribute `{}` can only accept string literals",
                    self.name
                )
            }
            AttrValue::Call(_) => {
                abort!(
                    self.name,
                    "attribute `{}` can only accept string literals",
                    self.name
                )
            }
        }
    }

    pub(crate) fn lit_bool_or_abort(&self) -> Result<bool, rust_syntax::Error> {
        let value = self.value_or_abort()?;
        match value {
            AttrValue::Expr(Expr::Lit(rust_syntax::ExprLit {
                lit: rust_syntax::Lit::Bool(lit),
                ..
            })) => Ok(lit.value()),
            _ => abort!(
                self.name,
                "attribute `{}` can only accept bool literals",
                self.name
            ),
        }
    }
}

impl Parse for CommandLineAttr {
    fn parse(input: ParseStream<'_>) -> rust_syntax::Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        let magic = match name_str.as_str() {
            "rename_all" => Some(MagicAttrName::RenameAll),
            "rename_all_env" => Some(MagicAttrName::RenameAllEnv),
            "skip" => Some(MagicAttrName::Skip),
            "next_display_order" => Some(MagicAttrName::NextDisplayOrder),
            "next_help_heading" => Some(MagicAttrName::NextHelpHeading),
            "default_value_t" => Some(MagicAttrName::DefaultValueT),
            "default_values_t" => Some(MagicAttrName::DefaultValuesT),
            "default_value_os_t" => Some(MagicAttrName::DefaultValueOsT),
            "default_values_os_t" => Some(MagicAttrName::DefaultValuesOsT),
            "long" => Some(MagicAttrName::Long),
            "short" => Some(MagicAttrName::Short),
            "value_parser" => Some(MagicAttrName::ValueParser),
            "action" => Some(MagicAttrName::Action),
            "env" => Some(MagicAttrName::Env),
            "flatten" => Some(MagicAttrName::Flatten),
            "value_enum" => Some(MagicAttrName::ValueEnum),
            "from_global" => Some(MagicAttrName::FromGlobal),
            "subcommand" => Some(MagicAttrName::Subcommand),
            "external_subcommand" => Some(MagicAttrName::ExternalSubcommand),
            "verbatim_doc_comment" => Some(MagicAttrName::VerbatimDocComment),
            "about" => Some(MagicAttrName::About),
            "long_about" => Some(MagicAttrName::LongAbout),
            "long_help" => Some(MagicAttrName::LongHelp),
            "author" => Some(MagicAttrName::Author),
            "version" => Some(MagicAttrName::Version),
            "defer" => Some(MagicAttrName::Defer),
            _ => None,
        };

        let value = if input.peek(Token![=]) {
            // `name = value` attributes.
            let assign_token = input.parse::<Token![=]>()?; // skip '='
            match input.parse::<Expr>() {
                Ok(expr) => Some(AttrValue::Expr(expr)),

                Err(_) => abort! {
                    assign_token,
                    "expected `string literal` or `expression` after `=`"
                },
            }
        } else if input.peek(rust_syntax::token::Paren) {
            // `name(...)` attributes.
            let nested;
            parenthesized!(nested in input);

            let method_args: Punctuated<_, _> = nested.parse_terminated(Expr::parse, Token![,])?;
            Some(AttrValue::Call(Vec::from_iter(method_args)))
        } else {
            None
        };

        Ok(Self {
            kind: Sp::new(AttrKind::Command, name.span()),
            name,
            magic,
            value,
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum MagicAttrName {
    Short,
    Long,
    ValueParser,
    Action,
    Env,
    Flatten,
    ValueEnum,
    FromGlobal,
    Subcommand,
    VerbatimDocComment,
    ExternalSubcommand,
    About,
    LongAbout,
    LongHelp,
    Author,
    Version,
    RenameAllEnv,
    RenameAll,
    Skip,
    DefaultValueT,
    DefaultValuesT,
    DefaultValueOsT,
    DefaultValuesOsT,
    NextDisplayOrder,
    NextHelpHeading,
    Defer,
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum AttrValue {
    Expr(Expr),
    Call(Vec<Expr>),
}

impl ToTokens for AttrValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Expr(t) => t.to_tokens(tokens),
            Self::Call(t) => {
                let t = quote!(#(#t),*);
                t.to_tokens(tokens);
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum AttrKind {
    Command,
    Group,
    Arg,
    Value,
}

impl AttrKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Group => "group",
            Self::Arg => "arg",
            Self::Value => "value",
        }
    }
}
