#[cfg(feature = "rust-syntax")]
#[test]
#[allow(unused)]
fn test_inference() {
    use rust_syntax::parse::Parse;

    if false {
        let wt = &WhatType::new();
        let ts: proc_macro::TokenStream = wt.manyhow_parse(quote::quote!(test), false).unwrap();
        let wt = &WhatType::new();
        if false {
            let wt: Result<rust_syntax::Ident, _> = wt.identify();
        }
        let ts: rust_syntax::Ident = wt.manyhow_parse(quote::quote!(test), false).unwrap();

        struct Parsable;
        impl Parse for Parsable {
            fn parse(input: rust_syntax::parse::ParseStream) -> rust_syntax::Result<Self> {
                todo!()
            }
        }
        let wt = &WhatType::new();
        let _: Result<Parsable, _> = wt.identify();
        let ts = wt.manyhow_parse(quote::quote!(test), false).unwrap();
    }
}
