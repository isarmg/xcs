#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use crate::syntax::ast::parse::Parser;

    fn assert_panic_message(pattern: &str, expected_msg: &str) {
        let result = Parser::new().parse(pattern);
        match result {
            Ok(_) => {
                panic!("regex should not have parsed");
            }
            Err(err) => {
                assert_eq!(err.to_string(), expected_msg.trim());
            }
        }
    }

    // See: https://github.com/rust-lang/regex/issues/464
    #[test]
    fn regression_464() {
        let err = Parser::new().parse("a{\n").unwrap_err();
        // This test checks that the error formatter doesn't panic.
        assert!(!err.to_string().is_empty());
    }

    // See: https://github.com/rust-lang/regex/issues/545
    #[test]
    fn repetition_quantifier_expects_a_valid_decimal() {
        assert_panic_message(
            r"\\u{[^}]*}",
            r#"
regex parse error:
    \\u{[^}]*}
        ^
error: repetition quantifier expects a valid decimal
"#,
        );
    }
}
