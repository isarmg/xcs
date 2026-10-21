//! Deserialization for untagged enums:
//!
//! ```ignore
//! #[serialization(untagged)]
//! enum Enum {}
//! ```

use crate::de::struct_;
use crate::de::tuple;
use crate::de::{
    effective_style, expr_is_missing, unwrap_to_variant_closure, Parameters, StructForm, TupleForm,
};
use crate::fragment::{Expr, Fragment};
use crate::internals::ast::{Field, Style, Variant};
use crate::internals::attr;
use crate::private;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

/// Generates `Deserialize::deserialize` body for an `enum Enum {...}` with `#[serialization(untagged)]` attribute
pub(super) fn deserialize(
    params: &Parameters,
    variants: &[Variant],
    cattrs: &attr::Container,
    first_attempt: Option<TokenStream>,
) -> Fragment {
    let attempts = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_deserializing())
        .map(|variant| Expr(deserialize_variant(params, variant, cattrs)));
    // TODO this message could be better by saving the errors from the failed
    // attempts. The heuristic used by TOML was to count the number of fields
    // processed before an error, and use the error that happened after the
    // largest number of fields. I'm not sure I like that. Maybe it would be
    // better to save all the errors and combine them into one message that
    // explains why none of the variants matched.
    let fallthrough_msg = format!(
        "data did not match any variant of untagged enum {}",
        params.type_name()
    );
    let fallthrough_msg = cattrs.expecting().unwrap_or(&fallthrough_msg);

    let private2 = private;
    quote_block! {
        let __content = _serialization::de::DeserializeSeed::deserialize(_serialization::#private::de::ContentVisitor::new(), __deserializer)?;
        let __deserializer = _serialization::#private::de::ContentRefDeserializer::<__D::Error>::new(&__content);

        #first_attempt

        #(
            if let _serialization::#private2::Ok(__ok) = #attempts {
                return _serialization::#private2::Ok(__ok);
            }
        )*

        _serialization::#private::Err(_serialization::de::Error::custom(#fallthrough_msg))
    }
}

// Also used by adjacently tagged enums
pub(super) fn deserialize_variant(
    params: &Parameters,
    variant: &Variant,
    cattrs: &attr::Container,
) -> Fragment {
    if let Some(path) = variant.attrs.deserialize_with() {
        let unwrap_fn = unwrap_to_variant_closure(params, variant, false);
        return quote_block! {
            _serialization::#private::Result::map(#path(__deserializer), #unwrap_fn)
        };
    }

    let variant_ident = &variant.ident;

    match effective_style(variant) {
        Style::Unit => {
            let this_value = &params.this_value;
            let type_name = params.type_name();
            let variant_name = variant.ident.to_string();
            let default = variant.fields.first().map(|field| {
                let default = Expr(expr_is_missing(field, cattrs));
                quote!((#default))
            });
            quote_expr! {
                match _serialization::Deserializer::deserialize_any(
                    __deserializer,
                    _serialization::#private::de::UntaggedUnitVisitor::new(#type_name, #variant_name)
                ) {
                    _serialization::#private::Ok(()) => _serialization::#private::Ok(#this_value::#variant_ident #default),
                    _serialization::#private::Err(__err) => _serialization::#private::Err(__err),
                }
            }
        }
        Style::Newtype => deserialize_newtype_variant(variant_ident, params, &variant.fields[0]),
        Style::Tuple => tuple::deserialize(
            params,
            &variant.fields,
            cattrs,
            TupleForm::Untagged(variant_ident),
        ),
        Style::Struct => struct_::deserialize(
            params,
            &variant.fields,
            cattrs,
            StructForm::Untagged(variant_ident),
        ),
    }
}

// Also used by internally tagged enums
// Implicitly (via `generate_variant`) used by adjacently tagged enums
pub(super) fn deserialize_newtype_variant(
    variant_ident: &syn::Ident,
    params: &Parameters,
    field: &Field,
) -> Fragment {
    let this_value = &params.this_value;
    let field_ty = field.ty;
    match field.attrs.deserialize_with() {
        None => {
            let span = field.original.span();
            let func = quote_spanned!(span=> <#field_ty as _serialization::Deserialize>::deserialize);
            quote_expr! {
                _serialization::#private::Result::map(#func(__deserializer), #this_value::#variant_ident)
            }
        }
        Some(path) => {
            quote_block! {
                let __value: _serialization::#private::Result<#field_ty, _> = #path(__deserializer);
                _serialization::#private::Result::map(__value, #this_value::#variant_ident)
            }
        }
    }
}
