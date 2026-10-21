#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::json;

    #[test]
    fn location() {
        tests_util::assert_schema_location(
            &json!({"uniqueItems": true}),
            &json!([1, 1]),
            "/uniqueItems",
        );
    }
}
