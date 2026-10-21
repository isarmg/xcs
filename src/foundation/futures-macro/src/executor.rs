use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned, ToTokens};

pub(crate) fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return rust_syntax::Error::new_spanned(proc_macro2::TokenStream::from(args), "invalid argument")
            .to_compile_error()
            .into();
    }

    let mut input = rust_syntax::parse_macro_input!(item as rust_syntax::ItemFn);

    if input.sig.asyncness.take().is_none() {
        return rust_syntax::Error::new_spanned(input.sig.fn_token, "Only async functions are supported")
            .to_compile_error()
            .into();
    }

    // If type mismatch occurs, the current rustc points to the last statement.
    let (last_stmt_start_span, last_stmt_end_span) = {
        let mut last_stmt = input
            .block
            .stmts
            .last()
            .map(ToTokens::into_token_stream)
            .unwrap_or_default()
            .into_iter();
        // `Span` on stable Rust has a limitation that only points to the first
        // token, not the whole tokens. We can work around this limitation by
        // using the first/last span of the tokens like
        // `rust_syntax::Error::new_spanned` does.
        let start = last_stmt.next().map_or_else(Span::call_site, |t| t.span());
        let end = last_stmt.last().map_or(start, |t| t.span());
        (start, end)
    };

    let path = quote_spanned! {last_stmt_start_span=>
        ::futures_test::__private
    };
    let body = &input.block;
    input.block.stmts = vec![rust_syntax::Stmt::Expr(
        rust_syntax::parse2(quote_spanned! {last_stmt_end_span=>
            #path::block_on(async #body)
        })
        .unwrap(),
        None,
    )];

    let gen = quote! {
        #[::core::prelude::v1::test]
        #input
    };

    gen.into()
}
