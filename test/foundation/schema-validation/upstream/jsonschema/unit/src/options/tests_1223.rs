#[cfg(test)]
mod tests {
    use super::*;
    use referencing::{Registry, Resource, Retrieve, Uri};
    use json_codec::{json, Value};
    use test_case::test_case;

    fn custom(s: &str) -> bool {
        s.ends_with("42!")
    }

    // An explicit `with_draft` replaces the declared `$schema`, so the vocabularies must come
    // from the draft actually in use, not from the declaration it overrides.
    #[test_case("http://json-schema.org/draft-04/schema#", Draft::Draft202012)]
    #[test_case("http://json-schema.org/draft-06/schema#", Draft::Draft202012)]
    #[test_case("http://json-schema.org/draft-07/schema#", Draft::Draft202012)]
    #[test_case("https://json-schema.org/draft/2019-09/schema", Draft::Draft202012)]
    #[test_case("http://json-schema.org/draft-04/schema#", Draft::Draft201909)]
    #[test_case("http://json-schema.org/draft-06/schema#", Draft::Draft201909)]
    #[test_case("http://json-schema.org/draft-07/schema#", Draft::Draft201909)]
    fn test_explicit_draft_keeps_keywords(declared: &str, forced: Draft) {
        let schema = json!({"$schema": declared, "type": "string", "minLength": 5});
        let validator = crate::options()
            .with_draft(forced)
            .build(&schema)
            .expect("a valid schema");
        assert!(!validator.is_valid(&json!(42)));
        assert!(!validator.is_valid(&json!("ab")));
        assert!(validator.is_valid(&json!("abcde")));
    }

    #[test_case(Draft::Draft7, "duration", false; "a format the draft does not carry")]
    #[test_case(Draft::Draft201909, "duration", true; "a format the draft carries")]
    #[test_case(Draft::Draft4, "json-pointer", false; "a pointer format below the draft that added it")]
    #[test_case(Draft::Draft202012, "date-time", true; "a format every draft carries")]
    #[test_case(Draft::Draft4, "hostname", true; "the draft 4 hostname check")]
    #[test_case(Draft::Draft202012, "unix-time", false; "a name no check answers to")]
    fn test_known_builtin_format(draft: Draft, name: &str, known: bool) {
        assert_eq!(crate::options().is_known_format(draft, name), known);
    }

    #[test]
    fn test_a_registered_format_is_known() {
        let options = crate::options().with_format("ends-with-42", custom);
        assert!(options.is_known_format(Draft::Draft202012, "ends-with-42"));
        assert!(!crate::options().is_known_format(Draft::Draft202012, "ends-with-42"));
    }

    #[cfg(feature = "idna")]
    #[test]
    fn test_an_international_format_is_known_with_its_feature() {
        assert!(crate::options().is_known_format(Draft::Draft7, "idn-email"));
    }

    #[cfg(not(feature = "idna"))]
    #[test]
    fn test_an_international_format_is_unknown_without_its_feature() {
        assert!(!crate::options().is_known_format(Draft::Draft7, "idn-email"));
    }

    #[test]
    fn test_offline_refuses_remote_references() {
        let schema = json!({"$ref": "https://example.com/schema.json"});
        let error = crate::options()
            .offline()
            .build(&schema)
            .expect_err("must refuse to fetch");
        assert_eq!(
            error.to_string(),
            "Resource 'https://example.com/schema.json' is not present in a registry \
             and retrieving it failed: Retrieval is disabled, cannot fetch \
             https://example.com/schema.json"
        );
    }

    #[cfg(all(feature = "resolve-async", not(target_family = "wasm")))]
    #[tokio::test]
    async fn test_async_offline_refuses_remote_references() {
        let schema = json!({"$ref": "https://example.com/schema.json"});
        let error = crate::async_options()
            .offline()
            .build(&schema)
            .await
            .expect_err("must refuse to fetch");
        assert_eq!(
            error.to_string(),
            "Resource 'https://example.com/schema.json' is not present in a registry \
             and retrieving it failed: Retrieval is disabled, cannot fetch \
             https://example.com/schema.json"
        );
    }

    #[test]
    fn custom_format() {
        let schema = json!({"type": "string", "format": "custom"});
        let validator = crate::options()
            .with_format("custom", custom)
            .should_validate_formats(true)
            .build(&schema)
            .expect("Valid schema");
        assert!(!validator.is_valid(&json!("foo")));
        assert!(validator.is_valid(&json!("foo42!")));
    }

    #[test]
    fn with_registry() {
        let registry = Registry::new()
            .add("urn:name-schema", json!({"type": "string"}))
            .expect("Invalid URI")
            .prepare()
            .expect("Registry should prepare");
        let schema = json!({
            "properties": {
                "name": { "$ref": "urn:name-schema" }
            }
        });
        let validator = crate::options()
            .with_registry(&registry)
            .build(&schema)
            .expect("Invalid schema");
        assert!(validator.is_valid(&json!({ "name": "Valid String" })));
        assert!(!validator.is_valid(&json!({ "name": 123 })));
    }

    struct InlineOnlyRetriever;

    impl Retrieve for InlineOnlyRetriever {
        fn retrieve(
            &self,
            uri: &Uri<String>,
        ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            if uri.as_str() == "https://example.com/string.json" {
                Ok(json!({"type": "string"}))
            } else {
                Err(format!("unexpected retrieval for {}", uri.as_str()).into())
            }
        }
    }

    #[test]
    fn with_registry_uses_validation_options_retriever_for_inline_only_refs() {
        let shared = Registry::new().prepare().expect("Registry should prepare");
        let schema = json!({"$ref": "https://example.com/string.json"});
        let validator = crate::options()
            .with_registry(&shared)
            .with_retriever(InlineOnlyRetriever)
            .build(&schema)
            .expect("Validator should build using the options retriever");

        assert!(validator.is_valid(&json!("Valid String")));
        assert!(!validator.is_valid(&json!(123)));
    }

    #[test]
    fn test_fancy_regex_options_builder() {
        let options = PatternOptions::fancy_regex()
            .backtrack_limit(1_000_000)
            .size_limit(10_000)
            .dfa_size_limit(5000);

        if let PatternEngineOptions::FancyRegex {
            backtrack_limit,
            size_limit,
            dfa_size_limit,
        } = options.inner
        {
            assert_eq!(backtrack_limit, Some(1_000_000));
            assert_eq!(size_limit, Some(10_000));
            assert_eq!(dfa_size_limit, Some(5000));
        } else {
            panic!("Expected FancyRegex variant");
        }
    }

    #[test]
    fn debug_lists_custom_content_checks() {
        let options = crate::options()
            .with_draft(Draft::Draft7)
            .with_content_media_type("application/x-a", |content: &str| content == "a");
        assert_eq!(
            format!("{options:?}"),
            "CompilationConfig { draft: Some(Draft7), content_media_type: [\"application/x-a\"], content_encoding: [] }"
        );
    }

    #[test]
    #[should_panic(expected = "Draft::Unknown is internal-only and cannot be explicitly set")]
    fn with_draft_rejects_unknown() {
        let _options = crate::options().with_draft(Draft::Unknown);
    }

    #[test]
    fn test_regex_options_builder() {
        let options = PatternOptions::regex()
            .size_limit(20_000)
            .dfa_size_limit(8000);

        if let PatternEngineOptions::Regex {
            size_limit,
            dfa_size_limit,
        } = options.inner
        {
            assert_eq!(size_limit, Some(20_000));
            assert_eq!(dfa_size_limit, Some(8000));
        } else {
            panic!("Expected Regex variant");
        }
    }

    #[test]
    fn test_validate_against_definition() {
        // Root schema with multiple definitions
        let root_schema = json!({
            "$id": "https://example.com/root",
            "definitions": {
                "User": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "integer", "minimum": 0}
                    },
                    "required": ["name"]
                },
                "Product": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"},
                        "title": {"type": "string"}
                    },
                    "required": ["id", "title"]
                }
            }
        });

        // Create a schema that references the specific definition
        let user_schema = json!({"$ref": "https://example.com/root#/definitions/User"});

        let registry = Registry::new()
            .add(
                "https://example.com/root",
                Resource::from_contents(root_schema),
            )
            .unwrap()
            .prepare()
            .unwrap();
        let validator = crate::options()
            .with_registry(&registry)
            .build(&user_schema)
            .expect("Valid schema");

        // Valid user
        assert!(validator.is_valid(&json!({"name": "Alice", "age": 30})));
        assert!(validator.is_valid(&json!({"name": "Bob"})));

        // Invalid: missing required field
        assert!(!validator.is_valid(&json!({"age": 25})));

        // Invalid: wrong type for age
        assert!(!validator.is_valid(&json!({"name": "Charlie", "age": "thirty"})));
    }
}
