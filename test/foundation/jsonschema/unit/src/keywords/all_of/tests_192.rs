#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"allOf": [{"type": "string"}]}), &json!(1), "/allOf/0/type")]
    #[test_case(&json!({"allOf": [{"type": "integer"}, {"maximum": 5}]}), &json!(6), "/allOf/1/maximum")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    #[test]
    fn malformed() {
        tests_util::assert_compile_error(
            &json!({"allOf": 5}),
            "5 is not of type \"array\"",
            "/allOf",
        );
    }
}
