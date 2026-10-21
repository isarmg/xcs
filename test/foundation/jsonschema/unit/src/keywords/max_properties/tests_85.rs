#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::json;

    #[test]
    fn location() {
        tests_util::assert_schema_location(
            &json!({"maxProperties": 1}),
            &json!({"a": 1, "b": 2}),
            "/maxProperties",
        );
    }
}
