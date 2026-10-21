#[cfg(test)]
mod tests {
    use super::*;
    use referencing::{Registry, Resource};
    use serde_json::json;

    use test_case::test_case;

    // `ValidationErrorIterator` requires `Send + Sync` (see its definition above); `LazyInstance`
    // uses `OnceLock` rather than `OnceCell` specifically to keep this holding once the repr can
    // carry a `Deferred` closure.
    const _: fn() = || {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ValidationError<'static>>();
    };

    fn owned_error(instance: Value, kind: ValidationErrorKind) -> ValidationError<'static> {
        ValidationError::new(
            LazyInstance::Ready(Cow::Owned(instance)),
            kind,
            Location::new(),
            Location::new(),
            Location::new(),
        )
    }

    #[test]
    fn test_validation_errors_display_includes_paths() {
        let schema = json!({
            "type": "object",
            "properties": {"age": {"type": "number"}},
            "required": ["name"]
        });
        let validator = crate::validator_for(&schema).expect("valid schema");
        let instance = json!({"age": "oops"});
        let errors = validator.iter_errors(&instance).into_errors();
        assert_eq!(
            errors.to_string(),
            "Validation errors:\n\
             01: \"name\" is a required property\n\
             02: /age: \"oops\" is not of type \"number\"\n"
        );
    }

    #[test]
    fn error_iterator_into_errors_collects_all_errors() {
        let iterator = ErrorIterator::from_iterator(
            vec![
                owned_error(json!(1), ValidationErrorKind::Minimum { limit: json!(2) }),
                owned_error(json!(3), ValidationErrorKind::Maximum { limit: json!(2) }),
            ]
            .into_iter(),
        );
        let validation_errors = iterator.into_errors();
        let collected: Vec<_> = validation_errors.into_iter().collect();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].to_string(), "1 is less than the minimum of 2");
        assert_eq!(
            collected[1].to_string(),
            "3 is greater than the maximum of 2"
        );
    }

    #[test]
    fn validation_errors_display_reports_success() {
        let errors = ValidationErrors { errors: Vec::new() };
        assert_eq!(format!("{errors}"), "Validation succeeded");
    }

    #[test]
    fn validation_errors_display_lists_messages() {
        let errors = ValidationErrors {
            errors: vec![
                owned_error(json!(1), ValidationErrorKind::Minimum { limit: json!(2) }),
                owned_error(json!(3), ValidationErrorKind::Maximum { limit: json!(2) }),
            ],
        };
        let rendered = format!("{errors}");
        assert!(rendered.contains("Validation errors:"));
        assert!(rendered.contains("01: 1 is less than the minimum of 2"));
        assert!(rendered.contains("02: 3 is greater than the maximum of 2"));
    }

    #[test]
    fn validation_errors_len_and_is_empty() {
        let empty = ValidationErrors { errors: vec![] };
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());

        let errors = ValidationErrors {
            errors: vec![owned_error(
                json!(1),
                ValidationErrorKind::Minimum { limit: json!(2) },
            )],
        };
        assert_eq!(errors.len(), 1);
        assert!(!errors.is_empty());
    }

    #[test]
    fn validation_errors_as_slice() {
        let errors = ValidationErrors {
            errors: vec![
                owned_error(json!(1), ValidationErrorKind::Minimum { limit: json!(2) }),
                owned_error(json!(3), ValidationErrorKind::Maximum { limit: json!(2) }),
            ],
        };

        let slice = errors.as_slice();
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0].to_string(), "1 is less than the minimum of 2");
        assert_eq!(slice[1].to_string(), "3 is greater than the maximum of 2");
    }

    #[test]
    fn validation_errors_iter() {
        let errors = ValidationErrors {
            errors: vec![
                owned_error(json!(1), ValidationErrorKind::Minimum { limit: json!(2) }),
                owned_error(json!(3), ValidationErrorKind::Maximum { limit: json!(2) }),
            ],
        };

        let collected: Vec<_> = errors.iter().map(ValidationError::to_string).collect();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0], "1 is less than the minimum of 2");
        assert_eq!(collected[1], "3 is greater than the maximum of 2");
    }

    #[test]
    #[allow(clippy::explicit_iter_loop)]
    fn validation_errors_iter_mut() {
        let mut errors = ValidationErrors {
            errors: vec![owned_error(
                json!(1),
                ValidationErrorKind::Minimum { limit: json!(2) },
            )],
        };

        // Verify we can get mutable references via iter_mut()
        for error in errors.iter_mut() {
            let _ = error.to_string();
        }
    }

    #[test]
    fn validation_errors_into_iterator_by_ref() {
        let errors = ValidationErrors {
            errors: vec![owned_error(
                json!(1),
                ValidationErrorKind::Minimum { limit: json!(2) },
            )],
        };

        let collected: Vec<_> = (&errors).into_iter().collect();
        assert_eq!(collected.len(), 1);
        // Verify errors is still usable
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn validation_errors_into_iterator_by_mut_ref() {
        let mut errors = ValidationErrors {
            errors: vec![owned_error(
                json!(1),
                ValidationErrorKind::Minimum { limit: json!(2) },
            )],
        };

        let collected: Vec<_> = (&mut errors).into_iter().collect();
        assert_eq!(collected.len(), 1);
        // Verify errors is still usable
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn error_iterator_size_hint() {
        let vec = vec![
            owned_error(json!(1), ValidationErrorKind::Minimum { limit: json!(2) }),
            owned_error(json!(3), ValidationErrorKind::Maximum { limit: json!(2) }),
        ];
        let iterator = ErrorIterator::from_iterator(vec.into_iter());
        let (lower, upper) = iterator.size_hint();
        assert_eq!(lower, 2);
        assert_eq!(upper, Some(2));
    }

    #[test]
    fn validation_errors_debug() {
        let errors = ValidationErrors {
            errors: vec![owned_error(
                json!(1),
                ValidationErrorKind::Minimum { limit: json!(2) },
            )],
        };
        let debug_output = format!("{errors:?}");
        assert!(debug_output.contains("ValidationErrors"));
        assert!(debug_output.contains("errors"));
    }

    #[test]
    fn single_type_error() {
        let instance = json!(42);
        let err = ValidationError::single_type_error(
            Location::new(),
            Location::new(),
            Location::new(),
            LazyInstance::Ready(Cow::Borrowed(&instance)),
            JsonType::String,
        );
        assert_eq!(err.to_string(), r#"42 is not of type "string""#);
    }

    #[test]
    fn multiple_types_error() {
        let instance = json!(42);
        let types = JsonTypeSet::from(JsonType::String).insert(JsonType::Number);
        let err = ValidationError::multiple_type_error(
            Location::new(),
            Location::new(),
            Location::new(),
            LazyInstance::Ready(Cow::Borrowed(&instance)),
            types,
        );
        assert_eq!(err.to_string(), r#"42 is not of types "number", "string""#);
    }

    #[test_case(true, &json!({"foo": {"bar": 42}}), "/foo/bar")]
    #[test_case(true, &json!({"foo": "a"}), "/foo")]
    #[test_case(false, &json!({"foo": {"bar": 42}}), "/foo/bar")]
    #[test_case(false, &json!({"foo": "a"}), "/foo")]
    fn instance_path_properties(additional_properties: bool, instance: &Value, expected: &str) {
        let schema = json!(
            {
                "additionalProperties": additional_properties,
                "type":"object",
                "properties":{
                   "foo":{
                      "type":"object",
                      "properties":{
                         "bar":{
                            "type":"string"
                         }
                      }
                   }
                }
            }
        );
        let validator = crate::validator_for(&schema).unwrap();
        let mut result = validator.iter_errors(instance);
        let error = result.next().expect("validation error");

        assert!(result.next().is_none());
        assert_eq!(error.instance_path().as_str(), expected);
    }

    #[test_case(true, &json!([1, {"foo": ["42"]}]), "/0")]
    #[test_case(true, &json!(["a", {"foo": [42]}]), "/1/foo/0")]
    #[test_case(false, &json!([1, {"foo": ["42"]}]), "/0")]
    #[test_case(false, &json!(["a", {"foo": [42]}]), "/1/foo/0")]
    fn instance_path_properties_and_arrays(
        additional_items: bool,
        instance: &Value,
        expected: &str,
    ) {
        let schema = json!(
            {
                "items": additional_items,
                "type": "array",
                "prefixItems": [
                    {
                        "type": "string"
                    },
                    {
                        "type": "object",
                        "properties": {
                            "foo": {
                                "type": "array",
                                "prefixItems": [
                                    {
                                        "type": "string"
                                    }
                                ]
                            }
                        }
                    }
                ]
            }
        );
        let validator = crate::validator_for(&schema).unwrap();
        let mut result = validator.iter_errors(instance);
        let error = result.next().expect("validation error");

        assert!(result.next().is_none());
        assert_eq!(error.instance_path().as_str(), expected);
    }

    #[test_case(true, &json!([[1, 2, 3], [4, "5", 6], [7, 8, 9]]), "/1/1")]
    #[test_case(false, &json!([[1, 2, 3], [4, "5", 6], [7, 8, 9]]), "/1/1")]
    #[test_case(true, &json!([[1, 2, 3], [4, 5, 6], 42]), "/2")]
    #[test_case(false, &json!([[1, 2, 3], [4, 5, 6], 42]), "/2")]
    fn instance_path_nested_arrays(additional_items: bool, instance: &Value, expected: &str) {
        let schema = json!(
            {
                "additionalItems": additional_items,
                "type": "array",
                "items": {
                    "type": "array",
                    "items": {
                        "type": "integer"
                    }
                }
            }
        );
        let validator = crate::validator_for(&schema).unwrap();
        let mut result = validator.iter_errors(instance);
        let error = result.next().expect("validation error");

        assert!(result.next().is_none());
        assert_eq!(error.instance_path().as_str(), expected);
    }

    #[test_case(true, &json!([1, "a"]), "/1")]
    #[test_case(false, &json!([1, "a"]), "/1")]
    #[test_case(true, &json!(123), "")]
    #[test_case(false, &json!(123), "")]
    fn instance_path_arrays(additional_items: bool, instance: &Value, expected: &str) {
        let schema = json!(
            {
                "additionalItems": additional_items,
                "type": "array",
                "items": {
                    "type": "integer"
                }
            }
        );
        let validator = crate::validator_for(&schema).unwrap();
        let mut result = validator.iter_errors(instance);
        let error = result.next().expect("validation error");

        assert!(result.next().is_none());
        assert_eq!(error.instance_path().as_str(), expected);
    }

    #[test_case(&json!({"anyOf": [{"type": "string"}, {"type": "boolean"}]}), &json!(1), "anyOf", "[REDACTED] is not valid under any of the schemas listed in the 'anyOf' keyword"; "any of")]
    #[test_case(&json!({"oneOf": [{"type": "string"}, {"type": "boolean"}]}), &json!(1), "oneOf", "[REDACTED] is not valid under any of the schemas listed in the 'oneOf' keyword"; "one of none valid")]
    #[test_case(&json!({"oneOf": [{"type": "integer"}, {"minimum": 0}]}), &json!(1), "oneOf", "[REDACTED] is valid under more than one of the schemas listed in the 'oneOf' keyword"; "one of multiple valid")]
    #[test_case(&json!({"contains": {"type": "string"}}), &json!([1]), "contains", "None of [REDACTED] are valid under the given schema"; "contains")]
    #[test_case(&json!({"const": "secret"}), &json!("x"), "const", "\"secret\" was expected"; "constant")]
    #[test_case(&json!({"$schema": "http://json-schema.org/draft-07/schema#", "contentEncoding": "base64"}), &json!("not base64!"), "contentEncoding", "[REDACTED] is not compliant with \"base64\" content encoding"; "content encoding")]
    #[test_case(&json!({"$schema": "http://json-schema.org/draft-07/schema#", "contentMediaType": "application/json"}), &json!("{"), "contentMediaType", "[REDACTED] is not compliant with \"application/json\" media type"; "content media type")]
    #[test_case(&json!({"$schema": "http://json-schema.org/draft-07/schema#", "contentMediaType": "application/json", "contentEncoding": "base64"}), &json!("/w=="), "contentEncoding", "invalid utf-8 sequence of 1 bytes from index 0"; "decoded content is not utf-8")]
    #[test_case(&json!({"enum": ["a", "b"]}), &json!("c"), "enum", "[REDACTED] is not one of \"a\" or \"b\""; "enumeration")]
    #[test_case(&json!({"exclusiveMaximum": 1}), &json!(1), "exclusiveMaximum", "[REDACTED] is greater than or equal to the maximum of 1"; "exclusive maximum")]
    #[test_case(&json!({"exclusiveMinimum": 1}), &json!(1), "exclusiveMinimum", "[REDACTED] is less than or equal to the minimum of 1"; "exclusive minimum")]
    #[test_case(&json!(false), &json!(1), "falseSchema", "False schema does not allow [REDACTED]"; "false schema")]
    #[test_case(&json!({"maximum": 1}), &json!(2), "maximum", "[REDACTED] is greater than the maximum of 1"; "maximum")]
    #[test_case(&json!({"maxLength": 1}), &json!("ab"), "maxLength", "[REDACTED] is longer than 1 character"; "max length singular")]
    #[test_case(&json!({"minLength": 1}), &json!(""), "minLength", "[REDACTED] is shorter than 1 character"; "min length singular")]
    #[test_case(&json!({"minLength": 2}), &json!("a"), "minLength", "[REDACTED] is shorter than 2 characters"; "min length plural")]
    #[test_case(&json!({"maxItems": 1}), &json!([1, 2]), "maxItems", "[REDACTED] has more than 1 item"; "max items singular")]
    #[test_case(&json!({"maxItems": 0}), &json!([1]), "maxItems", "[REDACTED] has more than 0 items"; "max items plural")]
    #[test_case(&json!({"minItems": 1}), &json!([]), "minItems", "[REDACTED] has less than 1 item"; "min items singular")]
    #[test_case(&json!({"minItems": 2}), &json!([1]), "minItems", "[REDACTED] has less than 2 items"; "min items plural")]
    #[test_case(&json!({"maxProperties": 1}), &json!({"a": 1, "b": 2}), "maxProperties", "[REDACTED] has more than 1 property"; "max properties singular")]
    #[test_case(&json!({"maxProperties": 0}), &json!({"a": 1}), "maxProperties", "[REDACTED] has more than 0 properties"; "max properties plural")]
    #[test_case(&json!({"minProperties": 1}), &json!({}), "minProperties", "[REDACTED] has less than 1 property"; "min properties singular")]
    #[test_case(&json!({"minProperties": 2}), &json!({"a": 1}), "minProperties", "[REDACTED] has less than 2 properties"; "min properties plural")]
    #[test_case(&json!({"not": {"type": "integer"}}), &json!(1), "not", "{\"type\":\"integer\"} is not allowed for [REDACTED]"; "not")]
    #[test_case(&json!({"propertyNames": {"maxLength": 1}}), &json!({"ab": 1}), "propertyNames", "\"ab\" is longer than 1 character"; "property names")]
    #[test_case(&json!({"required": ["a"]}), &json!({}), "required", "\"a\" is a required property"; "required")]
    #[test_case(&json!({"multipleOf": 2}), &json!(3), "multipleOf", "[REDACTED] is not a multiple of 2"; "multiple of")]
    #[test_case(&json!({"properties": {"a": {}}, "unevaluatedProperties": false}), &json!({"a": 1, "b": 2}), "unevaluatedProperties", "Unevaluated properties are not allowed ('b' was unexpected)"; "unevaluated properties")]
    #[test_case(&json!({"type": ["string", "null"]}), &json!(1), "type", "[REDACTED] is not of types \"null\", \"string\""; "multiple types")]
    #[test_case(&json!({"$schema": "http://json-schema.org/draft-04/schema#", "type": ["string", "null"]}), &json!(1), "type", "[REDACTED] is not of types \"null\", \"string\""; "multiple types draft 4")]
    fn masked_validation_messages(schema: &Value, instance: &Value, keyword: &str, expected: &str) {
        let validator = crate::validator_for(schema).expect("Invalid schema");
        let error = validator.validate(instance).expect_err("Should fail");
        assert_eq!(
            (
                error.kind().keyword(),
                error.masked_with("[REDACTED]").to_string()
            ),
            (keyword, expected.to_string())
        );
    }

    #[test]
    fn masked_unresolvable_reference() {
        let error = crate::validator_for(&json!({"$ref": "#/missing"})).expect_err("Should fail");
        assert_eq!(
            (error.kind().keyword(), error.masked().to_string()),
            ("$ref", "Pointer '/missing' does not exist".to_string())
        );
    }

    #[test]
    fn masked_backtrack_limit_exceeded() {
        let validator = crate::options()
            .with_pattern_options(crate::PatternOptions::fancy_regex().backtrack_limit(1))
            .build(&json!({"pattern": "(?<=ab)c"}))
            .expect("Invalid schema");
        let instance = json!("abc");
        let error = validator.validate(&instance).expect_err("Should fail");
        assert_eq!(
            (error.kind().keyword(), error.masked().to_string()),
            (
                "pattern",
                "Error executing regex: Max limit for backtracking count exceeded".to_string()
            )
        );
    }

    #[test_case(
        json!("2023-13-45"), 
        ValidationErrorKind::Format {
            format: "date".to_string(),
        },
        "value is not a \"date\""
    )]
    #[test_case(
        json!("sensitive data"),
        ValidationErrorKind::MaxLength { limit: 5 },
        "value is longer than 5 characters"
    )]
    #[test_case(
        json!({"secret": "data", "key": "value"}),
        ValidationErrorKind::AdditionalProperties {
            unexpected: vec!["secret".to_string(), "key".to_string()] 
        },
        "Additional properties are not allowed ('secret', 'key' were unexpected)"
    )]
    #[test_case(
        json!(123),
        ValidationErrorKind::Minimum { limit: json!(456) },
        "value is less than the minimum of 456"
    )]
    #[test_case(
        json!("secret_key_123"),
        ValidationErrorKind::Pattern {
            pattern: "^[A-Z0-9]{32}$".to_string(),
        },
        "value does not match \"^[A-Z0-9]{32}$\""
    )]
    #[test_case(
        json!([1, 2, 2, 3]),
        ValidationErrorKind::UniqueItems,
        "value has non-unique elements"
    )]
    #[test_case(
        json!(123),
        ValidationErrorKind::Type { kind: TypeKind::Single(JsonType::String) },
        "value is not of type \"string\""
    )]
    fn test_masked_error_messages(instance: Value, kind: ValidationErrorKind, expected: &str) {
        let error = ValidationError::new(
            LazyInstance::Ready(Cow::Owned(instance)),
            kind,
            Location::new(),
            Location::new(),
            Location::new(),
        );
        assert_eq!(error.masked().to_string(), expected);
    }

    #[test_case(
        json!("sensitive data"),
        ValidationErrorKind::MaxLength { limit: 5 },
        "[REDACTED]",
        "[REDACTED] is longer than 5 characters"
    )]
    #[test_case(
        json!({"password": "secret123"}),
        ValidationErrorKind::Type {
            kind: TypeKind::Single(JsonType::String)
        },
        "***",
        "*** is not of type \"string\""
    )]
    fn test_custom_masked_error_messages(
        instance: Value,
        kind: ValidationErrorKind,
        placeholder: &str,
        expected: &str,
    ) {
        let error = ValidationError::new(
            LazyInstance::Ready(Cow::Owned(instance)),
            kind,
            Location::new(),
            Location::new(),
            Location::new(),
        );
        assert_eq!(error.masked_with(placeholder).to_string(), expected);
    }

    fn single_error_messages(
        draft: referencing::Draft,
        schema: &Value,
        instance: &Value,
    ) -> (String, String, String) {
        let validator = crate::options()
            .with_draft(draft)
            .build(schema)
            .expect("schema compiles");
        let mut errors = validator.iter_errors(instance);
        let error = errors.next().expect("validation error");
        assert!(errors.next().is_none());
        (
            error.to_string(),
            error.masked().to_string(),
            error.masked_with("***").to_string(),
        )
    }

    #[test_case(referencing::Draft::Draft4, &json!([1, 2]), "Additional items are not allowed (2 was unexpected)", "Additional items are not allowed (1 item)"; "draft4 one extra")]
    #[test_case(referencing::Draft::Draft4, &json!([1, 2, 3]), "Additional items are not allowed (2, 3 were unexpected)", "Additional items are not allowed (2 items)"; "draft4 two extra")]
    #[test_case(referencing::Draft::Draft6, &json!([1, 2]), "Additional items are not allowed (2 was unexpected)", "Additional items are not allowed (1 item)"; "draft6 one extra")]
    #[test_case(referencing::Draft::Draft6, &json!([1, 2, 3]), "Additional items are not allowed (2, 3 were unexpected)", "Additional items are not allowed (2 items)"; "draft6 two extra")]
    #[test_case(referencing::Draft::Draft7, &json!([1, 2]), "Additional items are not allowed (2 was unexpected)", "Additional items are not allowed (1 item)"; "draft7 one extra")]
    #[test_case(referencing::Draft::Draft7, &json!([1, 2, 3]), "Additional items are not allowed (2, 3 were unexpected)", "Additional items are not allowed (2 items)"; "draft7 two extra")]
    #[test_case(referencing::Draft::Draft7, &json!([1, "a", null, 4]), "Additional items are not allowed (\"a\", null, 4 were unexpected)", "Additional items are not allowed (3 items)"; "draft7 three extra")]
    #[test_case(referencing::Draft::Draft201909, &json!([1, 2]), "Additional items are not allowed (2 was unexpected)", "Additional items are not allowed (1 item)"; "draft2019 one extra")]
    #[test_case(referencing::Draft::Draft201909, &json!([1, 2, 3]), "Additional items are not allowed (2, 3 were unexpected)", "Additional items are not allowed (2 items)"; "draft2019 two extra")]
    fn masked_additional_items_counts_extra_items(
        draft: referencing::Draft,
        instance: &Value,
        expected: &str,
        expected_masked: &str,
    ) {
        let schema = json!({"items": [{}], "additionalItems": false});
        let (message, masked, masked_with) = single_error_messages(draft, &schema, instance);
        assert_eq!(message, expected);
        assert_eq!(masked, expected_masked);
        assert_eq!(masked_with, expected_masked);
    }

    #[test_case(&json!([1, 2]), "Unevaluated items are not allowed ('2' was unexpected)", "Unevaluated items are not allowed (1 item)"; "one extra")]
    #[test_case(&json!([1, 2, 3]), "Unevaluated items are not allowed ('2', '3' were unexpected)", "Unevaluated items are not allowed (2 items)"; "two extra")]
    fn masked_unevaluated_items_pluralises_count(
        instance: &Value,
        expected: &str,
        expected_masked: &str,
    ) {
        let schema = json!({"prefixItems": [{}], "unevaluatedItems": false});
        let (message, masked, masked_with) =
            single_error_messages(referencing::Draft::Draft202012, &schema, instance);
        assert_eq!(message, expected);
        assert_eq!(masked, expected_masked);
        assert_eq!(masked_with, expected_masked);
    }

    #[test]
    fn test_absolute_keyword_location_absent_for_schema_without_base_uri() {
        let schema = serde_json::json!({"type": "string"});
        let instance = serde_json::json!(42);
        let err = crate::validate(&schema, &instance).unwrap_err();
        assert!(err.absolute_keyword_location().is_none());
    }

    #[test]
    fn test_absolute_keyword_location_present_for_schema_with_base_uri() {
        let schema = serde_json::json!({
            "$id": "https://example.com/schema.json",
            "type": "string"
        });
        let instance = serde_json::json!(42);
        let validator = crate::options()
            .build(&schema)
            .expect("schema should compile");
        let err = validator.validate(&instance).unwrap_err();
        let absolute_location = err
            .absolute_keyword_location()
            .expect("should have absolute keyword location");
        assert_eq!(
            absolute_location.as_str(),
            "https://example.com/schema.json#/type"
        );
    }

    #[test]
    fn test_absolute_keyword_location_from_iter_errors() {
        let schema = serde_json::json!({
            "$id": "https://example.com/schema.json",
            "type": "string"
        });
        let instance = serde_json::json!(42);
        let validator = crate::options()
            .build(&schema)
            .expect("schema should compile");
        let errs: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errs.len(), 1);
        let absolute_location = errs[0]
            .absolute_keyword_location()
            .expect("iter_errors should set absolute keyword location");
        assert_eq!(
            absolute_location.as_str(),
            "https://example.com/schema.json#/type"
        );
    }

    #[test]
    fn test_absolute_keyword_location_false_schema() {
        let schema = serde_json::json!(false);
        let instance = serde_json::json!(42);
        let validator = crate::options()
            .with_base_uri("https://example.com/schema.json")
            .build(&schema)
            .expect("schema should compile");
        let err = validator.validate(&instance).unwrap_err();
        let absolute_location = err
            .absolute_keyword_location()
            .expect("false schema should have absolute keyword location");
        assert_eq!(
            absolute_location.as_str(),
            "https://example.com/schema.json#"
        );
    }

    #[test]
    fn test_absolute_keyword_location_preserved_by_to_owned() {
        let schema = serde_json::json!({
            "$id": "https://example.com/schema.json",
            "type": "string"
        });
        let instance = serde_json::json!(42);
        let validator = crate::options()
            .build(&schema)
            .expect("schema should compile");
        let err = validator.validate(&instance).unwrap_err();
        let owned = err.to_owned();
        let absolute_location = owned
            .absolute_keyword_location()
            .expect("to_owned should preserve absolute keyword location");
        assert_eq!(
            absolute_location.as_str(),
            "https://example.com/schema.json#/type"
        );
    }

    #[test]
    fn test_absolute_keyword_location_external_ref() {
        let external = serde_json::json!({
            "$id": "https://example.com/string.json",
            "type": "string"
        });
        let schema = serde_json::json!({
            "$id": "https://example.com/root.json",
            "$ref": "https://example.com/string.json"
        });
        let instance = serde_json::json!(42);
        let registry = Registry::new()
            .add(
                "https://example.com/string.json",
                Resource::from_contents(external),
            )
            .expect("external schema should be accepted")
            .prepare()
            .expect("registry should build");
        let validator = crate::options()
            .with_registry(&registry)
            .build(&schema)
            .expect("schema should compile");
        let err = validator.validate(&instance).unwrap_err();
        let absolute_location = err
            .absolute_keyword_location()
            .expect("external ref error should have absolute keyword location");
        // Error originates in the external schema, not the root
        assert_eq!(
            absolute_location.as_str(),
            "https://example.com/string.json#/type"
        );
    }
}
