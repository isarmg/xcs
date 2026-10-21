#[cfg(feature = "syn")]
#[test]
#[allow(unused)]
fn test_inference() {
    use syn::parse::Parse;

    if false {
        let wt = &WhatType::new();
        let ts: proc_macro::TokenStream = wt.manyhow_parse(quote::quote!(test), false).unwrap();
        let wt = &WhatType::new();
        if false {
            let wt: Result<syn::Ident, _> = wt.identify();
        }
        let ts: syn::Ident = wt.manyhow_parse(quote::quote!(test), false).unwrap();

        struct Parsable;
        impl Parse for Parsable {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                todo!()
            }
        }
        let wt = &WhatType::new();
        let _: Result<Parsable, _> = wt.identify();
        let ts = wt.manyhow_parse(quote::quote!(test), false).unwrap();
    }
}
