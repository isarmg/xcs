#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::json;

    #[test]
    fn location() {
        tests_util::assert_schema_location(&json!({"maxItems": 1}), &json!([1, 2]), "/maxItems");
    }
}
