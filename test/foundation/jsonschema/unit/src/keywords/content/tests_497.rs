#[cfg(test)]
mod tests {
    use referencing::Draft;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"contentEncoding": "base64"}), &json!("asd"), "/contentEncoding")]
    #[test_case(&json!({"contentMediaType": "application/json"}), &json!("asd"), "/contentMediaType")]
    #[test_case(&json!({"contentMediaType": "application/json", "contentEncoding": "base64"}), &json!("ezp9Cg=="), "/contentMediaType")]
    #[test_case(&json!({"contentMediaType": "application/json", "contentEncoding": "base64"}), &json!("{}"), "/contentEncoding")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        let validator = crate::options()
            .with_draft(Draft::Draft7)
            .build(schema)
            .expect("Invalid schema");
        let error = validator.validate(instance).expect_err("Should fail");
        assert_eq!(error.schema_path().as_str(), expected);
    }

    #[test]
    fn invalid_utf8_after_base64_decode_has_content_encoding_location() {
        let schema = json!({
            "properties": {
                "data": {
                    "contentMediaType": "application/json",
                    "contentEncoding": "base64"
                }
            }
        });
        // "//4=" decodes to 0xFF 0xFE, which is not valid UTF-8
        let instance = json!({"data": "//4="});
        let validator = crate::options()
            .with_draft(Draft::Draft7)
            .build(&schema)
            .expect("Invalid schema");
        let error = validator.validate(&instance).expect_err("Should fail");
        assert_eq!(error.instance_path().as_str(), "/data");
        assert_eq!(
            error.schema_path().as_str(),
            "/properties/data/contentEncoding"
        );
        assert_eq!(error.instance().as_ref(), &json!("//4="));
        assert!(
            matches!(
                error.kind(),
                crate::error::ValidationErrorKind::FromUtf8 { .. }
            ),
            "expected FromUtf8, got {:?}",
            error.kind()
        );
    }

    #[test_case(crate::options(), &json!({"contentEncoding": "rot13"}), &json!("x"); "unknown encoding")]
    #[test_case(crate::options(), &json!({"contentEncoding": "rot13", "contentMediaType": "application/json"}), &json!("{"); "unknown encoding with media type")]
    #[test_case(crate::options(), &json!({"contentMediaType": "text/x-unknown"}), &json!("{"); "unknown media type")]
    #[test_case(crate::options().without_content_media_type_support("application/json"), &json!({"contentMediaType": "application/json"}), &json!("{"); "media type support removed")]
    #[test_case(crate::options().without_content_media_type_support("application/json"), &json!({"contentMediaType": "application/json", "contentEncoding": "base64"}), &json!("ew=="); "media type support removed with encoding")]
    #[test_case(crate::options().without_content_encoding_support("base64"), &json!({"contentEncoding": "base64"}), &json!("not base64!"); "encoding support removed")]
    #[test_case(crate::options().without_content_encoding_support("base64"), &json!({"contentEncoding": "base64", "contentMediaType": "application/json"}), &json!("not base64!"); "encoding support removed with media type")]
    fn unchecked(options: crate::ValidationOptions, schema: &Value, instance: &Value) {
        let validator = options
            .with_draft(Draft::Draft7)
            .build(schema)
            .expect("Invalid schema");
        crate::tests_util::is_valid_with(&validator, instance);
    }

    #[test_case(&json!({"contentMediaType": 5}), "/contentMediaType"; "media type")]
    #[test_case(&json!({"contentMediaType": "application/json", "contentEncoding": 5}), "/contentEncoding"; "encoding with media type")]
    #[test_case(&json!({"contentEncoding": 5}), "/contentEncoding"; "encoding")]
    fn malformed(schema: &Value, location: &str) {
        crate::tests_util::assert_compile_error_with(
            &crate::options().with_draft(Draft::Draft7),
            schema,
            "5 is not of type \"string\"",
            location,
        );
    }
}
