//! A Serde ast, parsed from the Rust Syntax ast and ready to generate Rust code.

use crate::internals::{attr, check, Ctxt, Derive};
use proc_macro2::Ident;
use rust_syntax::punctuated::Punctuated;
use rust_syntax::Token;

/// A source data structure annotated with `#[derive(Serialize)]` and/or `#[derive(Deserialize)]`,
/// parsed into an internal representation.
pub struct Container<'a> {
    /// The struct or enum name (without generics).
    pub ident: rust_syntax::Ident,
    /// Attributes on the structure, parsed for Serde.
    pub attrs: attr::Container,
    /// The contents of the struct or enum.
    pub data: Data<'a>,
    /// Any generics on the struct or enum.
    pub generics: &'a rust_syntax::Generics,
    /// Original input.
    pub original: &'a rust_syntax::DeriveInput,
}

/// The fields of a struct or enum.
///
/// Analogous to `rust_syntax::Data`.
pub enum Data<'a> {
    Enum(Vec<Variant<'a>>),
    Struct(Style, Vec<Field<'a>>),
}

/// A variant of an enum.
pub struct Variant<'a> {
    pub ident: rust_syntax::Ident,
    pub attrs: attr::Variant,
    pub style: Style,
    pub fields: Vec<Field<'a>>,
    pub original: &'a rust_syntax::Variant,
}

/// A field of a struct.
pub struct Field<'a> {
    pub member: rust_syntax::Member,
    pub attrs: attr::Field,
    pub ty: &'a rust_syntax::Type,
    pub original: &'a rust_syntax::Field,
}

#[derive(Copy, Clone)]
pub enum Style {
    /// Named fields.
    Struct,
    /// Many unnamed fields.
    Tuple,
    /// One unnamed field.
    Newtype,
    /// No fields.
    Unit,
}

impl<'a> Container<'a> {
    /// Convert the raw Rust Syntax ast into a parsed container object, collecting errors in `cx`.
    pub fn from_ast(
        cx: &Ctxt,
        item: &'a rust_syntax::DeriveInput,
        derive: Derive,
        private: &Ident,
    ) -> Option<Container<'a>> {
        let attrs = attr::Container::from_ast(cx, item);

        let mut data = match &item.data {
            rust_syntax::Data::Enum(data) => {
                Data::Enum(enum_from_ast(cx, &data.variants, attrs.default(), private))
            }
            rust_syntax::Data::Struct(data) => {
                let (style, fields) =
                    struct_from_ast(cx, &data.fields, None, attrs.default(), private);
                Data::Struct(style, fields)
            }
            rust_syntax::Data::Union(_) => {
                cx.error_spanned_by(item, "Serialization does not support derive for unions");
                return None;
            }
        };

        match &mut data {
            Data::Enum(variants) => {
                for variant in variants {
                    variant.attrs.rename_by_rules(attrs.rename_all_rules());
                    for field in &mut variant.fields {
                        field.attrs.rename_by_rules(
                            variant
                                .attrs
                                .rename_all_rules()
                                .or(attrs.rename_all_fields_rules()),
                        );
                    }
                }
            }
            Data::Struct(_, fields) => {
                for field in fields {
                    field.attrs.rename_by_rules(attrs.rename_all_rules());
                }
            }
        }

        let mut item = Container {
            ident: item.ident.clone(),
            attrs,
            data,
            generics: &item.generics,
            original: item,
        };
        check::check(cx, &mut item, derive);
        Some(item)
    }
}

impl<'a> Data<'a> {
    pub fn all_fields(&'a self) -> Box<dyn Iterator<Item = &'a Field<'a>> + 'a> {
        match self {
            Data::Enum(variants) => {
                Box::new(variants.iter().flat_map(|variant| variant.fields.iter()))
            }
            Data::Struct(_, fields) => Box::new(fields.iter()),
        }
    }

    pub fn has_getter(&self) -> bool {
        self.all_fields().any(|f| f.attrs.getter().is_some())
    }
}

fn enum_from_ast<'a>(
    cx: &Ctxt,
    variants: &'a Punctuated<rust_syntax::Variant, Token![,]>,
    container_default: &attr::Default,
    private: &Ident,
) -> Vec<Variant<'a>> {
    let variants: Vec<Variant> = variants
        .iter()
        .map(|variant| {
            let attrs = attr::Variant::from_ast(cx, variant);
            let (style, fields) = struct_from_ast(
                cx,
                &variant.fields,
                Some(&attrs),
                container_default,
                private,
            );
            Variant {
                ident: variant.ident.clone(),
                attrs,
                style,
                fields,
                original: variant,
            }
        })
        .collect();

    let index_of_last_tagged_variant = variants
        .iter()
        .rposition(|variant| !variant.attrs.untagged());
    if let Some(index_of_last_tagged_variant) = index_of_last_tagged_variant {
        for variant in &variants[..index_of_last_tagged_variant] {
            if variant.attrs.untagged() {
                cx.error_spanned_by(&variant.ident, "all variants with the #[serialization(untagged)] attribute must be placed at the end of the enum");
            }
        }
    }

    variants
}

fn struct_from_ast<'a>(
    cx: &Ctxt,
    fields: &'a rust_syntax::Fields,
    attrs: Option<&attr::Variant>,
    container_default: &attr::Default,
    private: &Ident,
) -> (Style, Vec<Field<'a>>) {
    match fields {
        rust_syntax::Fields::Named(fields) => (
            Style::Struct,
            fields_from_ast(cx, &fields.named, attrs, container_default, private),
        ),
        rust_syntax::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => (
            Style::Newtype,
            fields_from_ast(cx, &fields.unnamed, attrs, container_default, private),
        ),
        rust_syntax::Fields::Unnamed(fields) => (
            Style::Tuple,
            fields_from_ast(cx, &fields.unnamed, attrs, container_default, private),
        ),
        rust_syntax::Fields::Unit => (Style::Unit, Vec::new()),
    }
}

fn fields_from_ast<'a>(
    cx: &Ctxt,
    fields: &'a Punctuated<rust_syntax::Field, Token![,]>,
    attrs: Option<&attr::Variant>,
    container_default: &attr::Default,
    private: &Ident,
) -> Vec<Field<'a>> {
    let mut dst_fields = Vec::with_capacity(fields.len());
    for field in fields {
        dst_fields.push(Field {
            member: match &field.ident {
                Some(ident) => rust_syntax::Member::Named(ident.clone()),
                None => rust_syntax::Member::Unnamed(dst_fields.len().into()),
            },
            attrs: attr::Field::from_ast(
                cx,
                dst_fields.len(),
                field,
                attrs,
                container_default,
                private,
            ),
            ty: &field.ty,
            original: field,
        });
    }
    dst_fields
}
