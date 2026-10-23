use proc_macro2::{Ident, TokenStream};
use quote::quote_spanned;
use rust_syntax::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Field, ItemStruct, Token, Type,
};

use crate::attr_parsing::{combine_unary_attribute, parse_attrs, Combine};

pub(crate) fn expand(item: ItemStruct) -> rust_syntax::Result<TokenStream> {
    if !item.generics.params.is_empty() {
        return Err(rust_syntax::Error::new_spanned(
            item.generics,
            "`#[derive(FromRef)]` doesn't support generics",
        ));
    }

    let tokens = item
        .fields
        .iter()
        .enumerate()
        .map(|(idx, field)| expand_field(&item.ident, idx, field))
        .collect();

    Ok(tokens)
}

fn expand_field(state: &Ident, idx: usize, field: &Field) -> TokenStream {
    let FieldAttrs { skip } = match parse_attrs("from_ref", &field.attrs) {
        Ok(attrs) => attrs,
        Err(err) => return err.into_compile_error(),
    };

    if skip.is_some() {
        return TokenStream::default();
    }

    let field_ty = &field.ty;
    let span = field.ty.span();

    let body = if let Some(field_ident) = &field.ident {
        if matches!(field_ty, Type::Reference(_)) {
            quote_spanned! {span=> state.#field_ident }
        } else {
            quote_spanned! {span=> state.#field_ident.clone() }
        }
    } else {
        let idx = rust_syntax::Index {
            index: idx as _,
            span: field.span(),
        };
        quote_spanned! {span=> state.#idx.clone() }
    };

    quote_spanned! {span=>
        #[allow(clippy::clone_on_copy, clippy::clone_on_ref_ptr)]
        impl ::http_routing::extract::FromRef<#state> for #field_ty {
            fn from_ref(state: &#state) -> Self {
                #body
            }
        }
    }
}

mod kw {
    rust_syntax::custom_keyword!(skip);
}

#[derive(Default)]
pub(super) struct FieldAttrs {
    pub(super) skip: Option<kw::skip>,
}

impl Parse for FieldAttrs {
    fn parse(input: ParseStream<'_>) -> rust_syntax::Result<Self> {
        let mut skip = None;

        while !input.is_empty() {
            let lh = input.lookahead1();
            if lh.peek(kw::skip) {
                skip = Some(input.parse()?);
            } else {
                return Err(lh.error());
            }

            let _ = input.parse::<Token![,]>();
        }

        Ok(Self { skip })
    }
}

impl Combine for FieldAttrs {
    fn combine(mut self, other: Self) -> rust_syntax::Result<Self> {
        let Self { skip } = other;
        combine_unary_attribute(&mut self.skip, skip)?;
        Ok(self)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/axum-macros/functions/src/from_ref/ui_102.rs"));
