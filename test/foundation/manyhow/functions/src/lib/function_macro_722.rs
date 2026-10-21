#[test]
fn function_macro() {
    use proc_macro::TokenStream as TokenStream1;
    use quote::quote;
    // proc_macro2::TokenStream
    let output: TokenStream =
        function!(quote!(hello), |input: TokenStream| -> TokenStream { input });
    assert_eq!(output.to_string(), "hello");
    // proc_macro::TokenStream do not run :D
    if false {
        let _: TokenStream1 = function!(
            TokenStream1::from(quote!(hello)),
            |input: TokenStream1| -> TokenStream1 { input }
        );
    }

    #[cfg(feature = "syn")]
    {
        use quote::ToTokens;
        let output: TokenStream = function!(
            #[as_dummy]
            quote!(hello;),
            |input: syn::LitInt| -> TokenStream { input.into_token_stream() }
        );
        assert_eq!(
            output.to_string(),
            quote!(hello; ::core::compile_error! { "expected integer literal" }).to_string()
        );
        let output: TokenStream = function!(quote!(20), |_input: syn::LitInt| -> syn::Ident {
            syn::parse_quote!(hello)
        });
        assert_eq!(output.to_string(), "hello");
    }
}
