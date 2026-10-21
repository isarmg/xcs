#[cfg(test)]
mod test {
    use proc_macro_utils::assert_tokens;

    use super::*;

    #[test]
    fn error_message() {
        let error_message = ErrorMessage::new(Span::call_site(), "test message")
            .help("try to call your dog")
            .note("you could use the banana phone")
            .warning("be careful")
            .error("you cannot reach them");
        assert_tokens! {error_message.to_token_stream(), {
            ::core::compile_error! {
                "test message\n\n  = help: try to call your dog\n  = note: you could use the banana phone\n  = warning: be careful\n  = error: you cannot reach them\n"
            }
        }}
    }
}
