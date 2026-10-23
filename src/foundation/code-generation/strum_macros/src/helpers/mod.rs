pub use self::case_style::snakify;
pub use self::inner_variant_props::HasInnerVariantProperties;
pub use self::type_props::HasTypeProperties;
pub use self::variant_props::HasStrumVariantProperties;

pub mod case_style;
pub mod inner_variant_props;
mod metadata;
pub mod type_props;
pub mod variant_props;

use proc_macro2::Span;
use quote::ToTokens;
use rust_syntax::spanned::Spanned;

pub fn missing_parse_err_attr_error() -> rust_syntax::Error {
    rust_syntax::Error::new(
        Span::call_site(),
        "`parse_err_ty` and `parse_err_fn` attributes are both required.",
    )
}

pub fn non_enum_error() -> rust_syntax::Error {
    rust_syntax::Error::new(Span::call_site(), "This macro only supports enums.")
}

pub fn non_unit_variant_error() -> rust_syntax::Error {
    rust_syntax::Error::new(
        Span::call_site(),
        "This macro only supports enums of strictly unit variants. Consider \
        using it in conjunction with [`EnumDiscriminants`]",
    )
}

pub fn non_single_field_variant_error(attr: &str) -> rust_syntax::Error {
    rust_syntax::Error::new(
        Span::call_site(),
        format_args!(
            "The [`{}`] attribute only supports enum variants with a single field",
            attr
        ),
    )
}

pub fn strum_discriminants_passthrough_error(span: &impl Spanned) -> rust_syntax::Error {
    rust_syntax::Error::new(
        span.span(),
        "expected a pass-through attribute, e.g. #[strum_discriminants(serde(rename = \"var0\"))]",
    )
}

pub fn occurrence_error<T: ToTokens>(fst: T, snd: T, attr: &str) -> rust_syntax::Error {
    let mut e = rust_syntax::Error::new_spanned(
        snd,
        format!("Found multiple occurrences of strum({})", attr),
    );
    e.combine(rust_syntax::Error::new_spanned(fst, "first one here"));
    e
}
