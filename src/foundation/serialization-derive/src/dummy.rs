use proc_macro2::TokenStream;
use quote::quote;

pub fn wrap_in_const(serialization_path: Option<&syn::Path>, code: TokenStream) -> TokenStream {
    let use_serialization = match serialization_path {
        Some(path) => quote! {
            use #path as _serialization;
        },
        None => quote! {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate data_serialization as _serialization;
        },
    };

    quote! {
        #[doc(hidden)]
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_qualifications,
            clippy::absolute_paths,
        )]
        const _: () = {
            #use_serialization

            _serialization::__require_serialization_not_serialization_core!();

            #code
        };
    }
}
