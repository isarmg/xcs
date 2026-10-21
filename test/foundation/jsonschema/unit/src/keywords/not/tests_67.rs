#[cfg(test)]
mod tests {
    use crate::tests_util;
    use json_codec::json;

    #[test]
    fn location() {
        tests_util::assert_schema_location(
            &json!({"not": {"type": "string"}}),
            &json!("foo"),
            "/not",
        );
    }

    #[test]
    fn malformed() {
        tests_util::assert_compile_error(
            &json!({"not": 5}),
            "5 is not of types \"boolean\", \"object\"",
            "/not",
        );
    }
}
