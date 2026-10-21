#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::{Draft, Registry};

    use super::unescape_segment;
    use serde_json::json;
    use test_case::test_case;

    #[test_case("abc")]
    #[test_case("a~0b")]
    #[test_case("a~1b")]
    #[test_case("~01")]
    #[test_case("~10")]
    #[test_case("a~0~1b")]
    #[test_case("~"; "single tilde")]
    #[test_case("~~"; "double tilde")]
    #[test_case("~~~~~"; "many tildas")]
    #[test_case("~2")]
    #[test_case("a~c")]
    #[test_case("~0~1~")]
    #[test_case("")]
    #[test_case("a/d")]
    #[test_case("a~01b")]
    #[test_case("🌟~0🌠~1🌡️"; "Emojis with escapes")]
    #[test_case("~🌟"; "Tilde followed by emoji")]
    #[test_case("Café~0~1"; "Accented characters with escapes")]
    #[test_case("~é"; "Tilde followed by accented character")]
    #[test_case("αβγ"; "Greek")]
    #[test_case("~αβγ"; "Tilde followed by Greek")]
    #[test_case("∀∂∈ℝ∧∪≡∞"; "Mathematical symbols")]
    #[test_case("~∀∂∈ℝ∧∪≡∞"; "Tilde followed by mathematical symbols")]
    #[test_case("¡¢£¤¥¦§¨©"; "Special characters")]
    #[test_case("~¡¢£¤¥¦§¨©"; "Tilde followed by special characters")]
    #[test_case("\u{10FFFF}"; "Highest valid Unicode code point")]
    #[test_case("~\u{10FFFF}"; "Tilde followed by highest valid Unicode code point")]
    fn test_unescape_segment_equivalence(input: &str) {
        let unescaped = unescape_segment(input);
        let double_replaced = input.replace("~1", "/").replace("~0", "~");
        assert_eq!(unescaped, double_replaced, "Failed for: {input}");
    }

    fn create_test_registry() -> Registry<'static> {
        let schema = Draft::Draft202012.create_resource(json!({
            "type": "object",
            "properties": {
                "foo": { "type": "string" },
                "bar": { "type": "array", "items": [{"type": "number"}, {"type": "boolean"}] }
            }
        }));
        Registry::new()
            .add("http://example.com", schema)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources")
    }

    #[test]
    fn test_empty_ref() {
        let schema = Draft::Draft202012.create_resource(json!({
            "type": "object",
            "properties": {
                "foo": { "type": "string" }
            }
        }));
        let registry = Registry::new()
            .add("http://example.com", &schema)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));

        let resolved = resolver.lookup("#").expect("Lookup failed");
        assert_eq!(resolved.contents(), schema.contents());
    }

    #[test]
    fn test_percent_encoded_non_utf8() {
        let registry = create_test_registry();
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));

        let result = resolver.lookup("#/%FF");
        let error = result.expect_err("Should fail");
        assert_eq!(
            error.to_string(),
            "Invalid percent encoding in pointer '/%FF': the decoded bytes do not represent valid UTF-8"
        );
        assert!(error.source().is_some());
    }

    #[test]
    fn test_array_index_as_string() {
        let registry = create_test_registry();
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));

        let result = resolver.lookup("#/properties/bar/items/one");
        let error = result.expect_err("Should fail");
        assert_eq!(
            error.to_string(),
            "Failed to parse array index 'one' in pointer '/properties/bar/items/one'"
        );
        assert!(error.source().is_some());
    }

    #[test]
    fn test_array_index_out_of_bounds() {
        let registry = create_test_registry();
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));

        let result = resolver.lookup("#/properties/bar/items/2");
        assert_eq!(
            result.expect_err("Should fail").to_string(),
            "Pointer '/properties/bar/items/2' does not exist"
        );
    }

    #[test]
    fn test_unknown_property() {
        let registry = create_test_registry();
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));

        let result = resolver.lookup("#/properties/baz");
        assert_eq!(
            result.expect_err("Should fail").to_string(),
            "Pointer '/properties/baz' does not exist"
        );
    }
}
