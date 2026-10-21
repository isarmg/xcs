#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"dependencies": {"bar": ["foo"]}}), &json!({"bar": 1}), "/dependencies")]
    #[test_case(&json!({"dependencies": {"bar": {"type": "string"}}}), &json!({"bar": 1}), "/dependencies/bar/type")]
    #[test_case(&json!({"dependentRequired": {"bar": ["foo"]}}), &json!({"bar": 1}), "/dependentRequired")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    #[test_case(&json!({"a": 1, "b": 2, "c": 3}), &[]; "dependencies present")]
    #[test_case(&json!({"b": 2}), &[]; "trigger absent")]
    #[test_case(&json!([1]), &[]; "not an object")]
    #[test_case(&json!({"a": 1}), &["\"b\" is a required property", "\"c\" is a required property"]; "dependencies missing")]
    fn dependent_required(instance: &Value, expected: &[&str]) {
        let schema = json!({"dependentRequired": {"a": ["b", "c"]}});
        if expected.is_empty() {
            tests_util::is_valid(&schema, instance);
        } else {
            tests_util::is_not_valid(&schema, instance);
        }
        tests_util::expect_errors(&schema, instance, expected);
    }

    #[test_case(&json!({"dependentRequired": 5}), "5 is not of type \"object\"", "/dependentRequired"; "dependent required not object")]
    #[test_case(&json!({"dependentRequired": {"a": 5}}), "5 is not of type \"array\"", "/dependentRequired/a"; "dependent required entry not array")]
    #[test_case(&json!({"dependentRequired": {"a": ["b", "b"]}}), "[\"b\",\"b\"] has non-unique elements", "/dependentRequired/a"; "dependent required entry not unique")]
    #[test_case(&json!({"dependentSchemas": 5}), "5 is not of type \"object\"", "/dependentSchemas"; "dependent schemas not object")]
    fn malformed(schema: &Value, message: &str, location: &str) {
        tests_util::assert_compile_error(schema, message, location);
    }

    #[test_case(&json!({"dependencies": 5}), "5 is not of type \"object\""; "not object")]
    #[test_case(&json!({"dependencies": {"a": [1]}}), "1 is not of type \"string\""; "entry not string")]
    fn malformed_dependencies(schema: &Value, message: &str) {
        tests_util::assert_compile_error_with(
            &crate::options().with_draft(crate::Draft::Draft7),
            schema,
            message,
            "/dependencies",
        );
    }
}
