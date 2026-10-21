pub(crate) trait SpanError {
    #[allow(non_snake_case)]
    fn EXPECTED_Span_OR_ToTokens<D: std::fmt::Display>(&self, msg: D) -> rust_syntax::Error;
}

pub(crate) trait ToTokensError {
    #[allow(non_snake_case)]
    fn EXPECTED_Span_OR_ToTokens<D: std::fmt::Display>(&self, msg: D) -> rust_syntax::Error;
}

impl<T: quote::ToTokens> ToTokensError for T {
    fn EXPECTED_Span_OR_ToTokens<D: std::fmt::Display>(&self, msg: D) -> rust_syntax::Error {
        // Curb monomorphization from generating too many identical `new_spanned`.
        rust_syntax::Error::new_spanned(self.to_token_stream(), msg)
    }
}

impl SpanError for proc_macro2::Span {
    fn EXPECTED_Span_OR_ToTokens<D: std::fmt::Display>(&self, msg: D) -> rust_syntax::Error {
        rust_syntax::Error::new(*self, msg)
    }
}
