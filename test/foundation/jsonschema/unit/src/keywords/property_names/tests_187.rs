#[cfg(test)]
mod tests {
    use crate::tests_util;
    use json_codec::{json, Value};
    use test_case::test_case;

    // Each key is validated on its own, not concatenated with the ones before it
    #[test_case(&json!({"propertyNames": {"maxLength": 3}}), &json!({"abc": 1, "de": 1}))]
    #[test_case(&json!({"propertyNames": {"maxLength": 3}}), &json!({"a": 1, "bc": 1, "def": 1}))]
    fn keys_validate_independently(schema: &Value, instance: &Value) {
        tests_util::is_valid(schema, instance);
    }

    #[test_case(&json!({"propertyNames": {"maxLength": 2}}), &json!({"ab": 1, "cde": 1}))]
    #[test_case(&json!({"propertyNames": {"minLength": 2}}), &json!({"ab": 1, "c": 1}))]
    fn invalid_key_is_reported(schema: &Value, instance: &Value) {
        tests_util::is_not_valid(schema, instance);
    }

    #[test_case(&json!({"propertyNames": false}), &json!({"foo": 1}), "/propertyNames")]
    #[test_case(&json!({"propertyNames": {"minLength": 2}}), &json!({"f": 1}), "/propertyNames/minLength")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }
}
