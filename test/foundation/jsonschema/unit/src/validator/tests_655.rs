#[cfg(test)]
mod tests {
    use crate::{
        error::ValidationError, keywords::custom::Keyword, paths::Location, Draft, Validator,
        ValidatorMap,
    };
    use fancy_regex::Regex;
    use num_cmp::NumCmp;
    use serde_json::{json, Map, Value};
    use std::sync::LazyLock;
    use test_case::test_case;

    #[cfg(not(target_arch = "wasm32"))]
    fn load(path: &str, idx: usize) -> Value {
        use std::{fs::File, io::Read, path::Path};
        let path = Path::new(path);
        let mut file = File::open(path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).ok().unwrap();
        let data: Value = serde_json::from_str(&content).unwrap();
        let case = &data.as_array().unwrap()[idx];
        case.get("schema").unwrap().clone()
    }

    /// Large enough that early paths are built directly and later ones come from the cache.
    #[test]
    fn evaluation_paths_match_across_cache_threshold() {
        const ELEMENTS: usize = 5000;

        let schema = json!({
            "type": "array",
            "items": {"$ref": "#/$defs/entry"},
            "$defs": {
                "entry": {
                    "type": "object",
                    "properties": {"value": {"$ref": "#/$defs/leaf"}}
                },
                "leaf": {"type": "integer"}
            }
        });
        let validator = crate::validator_for(&schema).expect("Valid schema");
        let instance = Value::Array((0..ELEMENTS).map(|_| json!({"value": "no"})).collect());

        let evaluation = validator.evaluate(&instance);
        let list = serde_json::to_value(evaluation.list()).expect("Serializable");
        let failures: Vec<_> = list["details"]
            .as_array()
            .expect("`details` is an array")
            .iter()
            .filter(|unit| unit.get("errors").is_some())
            .collect();

        assert_eq!(failures.len(), ELEMENTS);
        for (idx, unit) in failures.iter().enumerate() {
            assert_eq!(
                unit["evaluationPath"],
                "/items/$ref/properties/value/$ref/type"
            );
            assert_eq!(unit["instanceLocation"], format!("/{idx}/value"));
            assert_eq!(unit["schemaLocation"], "/$defs/leaf/type");
        }
    }

    #[test]
    fn debug_shows_root_and_draft() {
        let validator = crate::validator_for(&json!(true)).expect("Valid schema");
        assert_eq!(
            format!("{validator:?}"),
            r#"Validator { root: SchemaNode { inner: SchemaNodeInner { validators: Boolean, .. }, location: Location(""), .. }, draft: Draft202012, .. }"#
        );
    }

    fn schema_declaring(uri: Option<&str>) -> Value {
        match uri {
            Some(uri) => json!({"$schema": uri, "type": "string"}),
            None => json!({"type": "string"}),
        }
    }

    #[test_case(None, None, Draft::Draft202012; "no schema default")]
    #[test_case(Some("http://json-schema.org/draft-04/schema#"), None, Draft::Draft4; "draft 4")]
    #[test_case(Some("http://json-schema.org/draft-06/schema#"), None, Draft::Draft6; "draft 6")]
    #[test_case(Some("http://json-schema.org/draft-07/schema#"), None, Draft::Draft7; "draft 7")]
    #[test_case(Some("https://json-schema.org/draft/2019-09/schema"), None, Draft::Draft201909; "draft 2019-09")]
    #[test_case(Some("https://json-schema.org/draft/2020-12/schema"), None, Draft::Draft202012; "draft 2020-12")]
    #[test_case(Some("https://json-schema.org/schema"), None, Draft::Draft202012; "version-less")]
    #[test_case(None, Some(Draft::Draft4), Draft::Draft4; "no schema explicit draft 4")]
    #[test_case(Some("http://json-schema.org/draft-04/schema#"), Some(Draft::Draft4), Draft::Draft4; "explicit draft 4 matching")]
    #[test_case(Some("http://json-schema.org/draft-07/schema#"), Some(Draft::Draft4), Draft::Draft4; "explicit draft 4 over draft 7")]
    #[test_case(Some("http://json-schema.org/draft-04/schema#"), Some(Draft::Draft202012), Draft::Draft202012; "explicit draft 2020-12 over draft 4")]
    #[test_case(Some("https://json-schema.org/draft/2020-12/schema"), Some(Draft::Draft6), Draft::Draft6; "explicit draft 6 over draft 2020-12")]
    fn reports_draft_used_to_validate(uri: Option<&str>, explicit: Option<Draft>, expected: Draft) {
        let schema = schema_declaring(uri);
        let mut options = crate::options();
        if let Some(draft) = explicit {
            options = options.with_draft(draft);
        }
        let validator = options.build(&schema).expect("Valid schema");
        let map = options.build_map(&schema).expect("Valid schema");
        let root = map.get("#").expect("Root is present");
        assert_eq!([validator.draft(), root.draft()], [expected; 2]);
    }

    #[test]
    fn validator_for_reports_declared_draft() {
        let schema = schema_declaring(Some("http://json-schema.org/draft-04/schema#"));
        let validator = crate::validator_for(&schema).expect("Valid schema");
        assert_eq!(validator.draft(), Draft::Draft4);
    }

    #[test]
    fn draft_constructors_report_their_draft() {
        let schema = schema_declaring(Some("https://json-schema.org/draft/2020-12/schema"));
        let drafts = [
            crate::draft4::new(&schema).expect("Valid schema").draft(),
            crate::draft6::new(&schema).expect("Valid schema").draft(),
            crate::draft7::new(&schema).expect("Valid schema").draft(),
            crate::draft201909::new(&schema)
                .expect("Valid schema")
                .draft(),
            crate::draft202012::new(&schema)
                .expect("Valid schema")
                .draft(),
        ];
        assert_eq!(
            drafts,
            [
                Draft::Draft4,
                Draft::Draft6,
                Draft::Draft7,
                Draft::Draft201909,
                Draft::Draft202012
            ]
        );
    }

    #[test]
    fn meta_validators_report_their_draft() {
        let drafts = [
            crate::draft4::meta::validator().draft(),
            crate::draft6::meta::validator().draft(),
            crate::draft7::meta::validator().draft(),
            crate::draft201909::meta::validator().draft(),
            crate::draft202012::meta::validator().draft(),
        ];
        assert_eq!(
            drafts,
            [
                Draft::Draft4,
                Draft::Draft6,
                Draft::Draft7,
                Draft::Draft201909,
                Draft::Draft202012
            ]
        );
    }

    #[test]
    fn custom_meta_schema_reports_the_draft_it_builds_on() {
        let meta_schema = crate::Resource::from_contents(json!({
            "$id": "https://example.com/meta",
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object"
        }));
        let registry = crate::Registry::new()
            .add("https://example.com/meta", meta_schema)
            .expect("Valid resource")
            .prepare()
            .expect("Valid registry");
        let schema = schema_declaring(Some("https://example.com/meta"));
        let validator = crate::options()
            .with_registry(&registry)
            .build(&schema)
            .expect("Valid schema");
        assert_eq!(validator.draft(), Draft::Draft7);
    }

    #[test]
    fn unknown_meta_schema_is_rejected() {
        let schema = schema_declaring(Some("https://example.com/unknown"));
        assert!(crate::options().offline().build(&schema).is_err());
    }

    #[cfg(all(feature = "resolve-async", not(target_family = "wasm")))]
    #[tokio::test]
    async fn async_build_reports_declared_draft() {
        let schema = schema_declaring(Some("http://json-schema.org/draft-04/schema#"));
        let validator = crate::async_validator_for(&schema)
            .await
            .expect("Valid schema");
        assert_eq!(validator.draft(), Draft::Draft4);
    }

    #[test]
    fn only_keyword() {
        // When only one keyword is specified
        let schema = json!({"type": "string"});
        let validator = crate::validator_for(&schema).unwrap();
        let value1 = json!("AB");
        let value2 = json!(1);
        // And only this validator
        assert_eq!(validator.root.validators().len(), 1);
        assert!(validator.validate(&value1).is_ok());
        assert!(validator.validate(&value2).is_err());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn validate_ref() {
        let schema = load("../../../test/foundation/jsonschema/tests/suite/tests/draft7/ref.json", 1);
        let value = json!({"bar": 3});
        let validator = crate::validator_for(&schema).unwrap();
        assert!(validator.validate(&value).is_ok());
        let value = json!({"bar": true});
        assert!(validator.validate(&value).is_err());
    }

    #[test]
    fn wrong_schema_type() {
        let schema = json!([1]);
        let validator = crate::validator_for(&schema);
        assert!(validator.is_err());
    }

    #[test]
    fn multiple_errors() {
        let schema = json!({"minProperties": 2, "propertyNames": {"minLength": 3}});
        let value = json!({"a": 3});
        let validator = crate::validator_for(&schema).unwrap();
        let errors: Vec<_> = validator.iter_errors(&value).collect();
        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors[0].to_string(),
            r#"{"a":3} has less than 2 properties"#
        );
        assert_eq!(errors[1].to_string(), r#""a" is shorter than 3 characters"#);
    }

    #[test]
    fn custom_keyword_definition() {
        // Define a custom validator that verifies the object's keys consist of
        // only ASCII representable characters.
        // NOTE: This could be done with `propertyNames` + `pattern` but will be slower due to
        // regex usage.
        struct CustomObjectValidator;
        impl<'i> Keyword<'i> for CustomObjectValidator {
            fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
                for key in instance.as_object().unwrap().keys() {
                    if !key.is_ascii() {
                        return Err(ValidationError::custom("Key is not ASCII"));
                    }
                }
                Ok(())
            }

            fn is_valid(&self, instance: &'i Value) -> bool {
                for (key, _value) in instance.as_object().unwrap() {
                    if !key.is_ascii() {
                        return false;
                    }
                }
                true
            }
        }

        fn custom_object_type_factory<'a>(
            _: &'a Map<String, Value>,
            schema: &'a Value,
            _path: Location,
        ) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
            const EXPECTED: &str = "ascii-keys";
            if schema.as_str() == Some(EXPECTED) {
                Ok(Box::new(CustomObjectValidator))
            } else {
                Err(ValidationError::schema(format!(
                    "Expected '{EXPECTED}', got {schema}"
                )))
            }
        }

        // Define a JSON schema that enforces the top level object has ASCII keys and has at least 1 property
        let schema =
            json!({ "custom-object-type": "ascii-keys", "type": "object", "minProperties": 1 });
        let validator = crate::options()
            .with_keyword("custom-object-type", custom_object_type_factory)
            .build(&schema)
            .unwrap();

        // Verify schema validation detects object with too few properties
        let instance = json!({});
        assert!(validator.validate(&instance).is_err());
        assert!(!validator.is_valid(&instance));

        // Verify validator succeeds on a valid custom-object-type
        let instance = json!({ "a" : 1 });
        assert!(validator.validate(&instance).is_ok());
        assert!(validator.is_valid(&instance));

        // Verify validator detects invalid custom-object-type
        let instance = json!({ "å" : 1 });
        let error = validator.validate(&instance).expect_err("Should fail");
        assert_eq!(error.to_string(), "Key is not ASCII");
        assert!(!validator.is_valid(&instance));
    }

    #[test]
    fn custom_keyword_iter_errors() {
        // A custom keyword that reports every non-ASCII key at once via `iter_errors`.
        struct NonAsciiKeys;
        impl<'i> Keyword<'i> for NonAsciiKeys {
            fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
                self.iter_errors(instance).next().map_or(Ok(()), Err)
            }

            fn is_valid(&self, instance: &'i Value) -> bool {
                instance
                    .as_object()
                    .is_none_or(|obj| obj.keys().all(|key| key.is_ascii()))
            }

            fn iter_errors(
                &self,
                instance: &'i Value,
            ) -> Box<dyn Iterator<Item = ValidationError<'i>> + 'i> {
                let errors: Vec<_> = instance
                    .as_object()
                    .into_iter()
                    .flat_map(|obj| obj.keys())
                    .filter(|key| !key.is_ascii())
                    .map(|key| ValidationError::custom(format!("Key {key:?} is not ASCII")))
                    .collect();
                Box::new(errors.into_iter())
            }
        }

        fn factory<'a>(
            _: &'a Map<String, Value>,
            _: &'a Value,
            _: Location,
        ) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
            Ok(Box::new(NonAsciiKeys))
        }

        let schema = json!({ "ascii-keys": true });
        let validator = crate::options()
            .with_keyword("ascii-keys", factory)
            .build(&schema)
            .unwrap();

        let instance = json!({ "å": 1, "ø": 2, "ok": 3 });
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].to_string(), r#"Key "å" is not ASCII"#);
        assert_eq!(errors[1].to_string(), r#"Key "ø" is not ASCII"#);

        // `validate` yields the first error; `is_valid` short-circuits.
        assert!(!validator.is_valid(&instance));
        assert_eq!(
            validator.validate(&instance).unwrap_err().to_string(),
            r#"Key "å" is not ASCII"#
        );

        let valid = json!({ "ok": 1 });
        assert!(validator.is_valid(&valid));
        assert!(validator.validate(&valid).is_ok());
    }

    #[test]
    fn custom_keyword_iter_errors_default() {
        // A keyword that does NOT override `iter_errors`, exercising the default
        // that yields at most one error from `validate`.
        struct NonEmpty;
        impl<'i> Keyword<'i> for NonEmpty {
            fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
                if self.is_valid(instance) {
                    Ok(())
                } else {
                    Err(ValidationError::custom("must not be empty"))
                }
            }

            fn is_valid(&self, instance: &'i Value) -> bool {
                instance.as_object().is_none_or(|obj| !obj.is_empty())
            }
        }

        fn factory<'a>(
            _: &'a Map<String, Value>,
            _: &'a Value,
            _: Location,
        ) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
            Ok(Box::new(NonEmpty))
        }

        let validator = crate::options()
            .with_keyword("non-empty", factory)
            .build(&json!({ "non-empty": true }))
            .unwrap();

        let empty = json!({});
        let errors: Vec<_> = validator.iter_errors(&empty).collect();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].to_string(), "must not be empty");

        let non_empty = json!({ "a": 1 });
        assert_eq!(validator.iter_errors(&non_empty).count(), 0);
    }

    #[test]
    fn custom_format_and_override_keyword() {
        // Check that a string has some number of digits followed by a dot followed by exactly 2 digits.
        fn currency_format_checker(s: &str) -> bool {
            static CURRENCY_RE: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new("^(0|([1-9]+[0-9]*))(\\.[0-9]{2})$").expect("Invalid regex")
            });
            CURRENCY_RE.is_match(s).expect("Invalid regex")
        }
        // A custom keyword validator that overrides "minimum"
        // so that "minimum" may apply to "currency"-formatted strings as well.
        struct CustomMinimumValidator {
            limit: f64,
            with_currency_format: bool,
        }

        impl<'i> Keyword<'i> for CustomMinimumValidator {
            fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
                if self.is_valid(instance) {
                    Ok(())
                } else {
                    Err(ValidationError::custom(format!(
                        "value is less than the minimum of {}",
                        self.limit
                    )))
                }
            }

            fn is_valid(&self, instance: &'i Value) -> bool {
                match instance {
                    // Numeric comparison should happen just like original behavior
                    Value::Number(instance) => {
                        if let Some(item) = instance.as_u64() {
                            !NumCmp::num_lt(item, self.limit)
                        } else if let Some(item) = instance.as_i64() {
                            !NumCmp::num_lt(item, self.limit)
                        } else {
                            let item = instance.as_f64().expect("Always valid");
                            !NumCmp::num_lt(item, self.limit)
                        }
                    }
                    // String comparison should cast currency-formatted
                    Value::String(instance)
                        if self.with_currency_format && currency_format_checker(instance) =>
                    {
                        // all preconditions for minimum applying are met
                        let value = instance
                            .parse::<f64>()
                            .expect("format validated by regex checker");
                        !NumCmp::num_lt(value, self.limit)
                    }
                    // In all other cases, the "minimum" keyword should not apply
                    _ => true,
                }
            }
        }

        // Build a validator that overrides the standard `minimum` keyword
        fn custom_minimum_factory<'a>(
            parent: &'a Map<String, Value>,
            schema: &'a Value,
            _path: Location,
        ) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
            let limit = if let Value::Number(limit) = schema {
                limit.as_f64().expect("Always valid")
            } else {
                return Err(ValidationError::schema("minimum must be a number"));
            };
            let with_currency_format = parent
                .get("format")
                .is_some_and(|format| format == "currency");
            Ok(Box::new(CustomMinimumValidator {
                limit,
                with_currency_format,
            }))
        }

        // Schema includes both the custom format and the overridden keyword
        let schema = json!({ "minimum": 2, "type": "string", "format": "currency" });
        let validator = crate::options()
            .with_format("currency", currency_format_checker)
            .with_keyword("minimum", custom_minimum_factory)
            .with_keyword("minimum-2", custom_minimum_factory)
            .should_validate_formats(true)
            .build(&schema)
            .expect("Invalid schema");

        // Control: verify schema validation rejects non-string types
        let instance = json!(15);
        assert!(validator.validate(&instance).is_err());
        assert!(!validator.is_valid(&instance));

        // Control: verify validator rejects ill-formatted strings
        let instance = json!("not a currency");
        assert!(validator.validate(&instance).is_err());
        assert!(!validator.is_valid(&instance));

        // Verify validator allows properly formatted strings that conform to custom keyword
        let instance = json!("3.00");
        assert!(validator.validate(&instance).is_ok());
        assert!(validator.is_valid(&instance));

        // Verify validator rejects properly formatted strings that do not conform to custom keyword
        let instance = json!("1.99");
        assert!(validator.validate(&instance).is_err());
        assert!(!validator.is_valid(&instance));

        // Define another schema that applies "minimum" to an integer to ensure original behavior
        let schema = json!({ "minimum": 2, "type": "integer" });
        let validator = crate::options()
            .with_format("currency", currency_format_checker)
            .with_keyword("minimum", custom_minimum_factory)
            .build(&schema)
            .expect("Invalid schema");

        // Verify schema allows integers greater than 2
        let instance = json!(3);
        assert!(validator.validate(&instance).is_ok());
        assert!(validator.is_valid(&instance));

        // Verify schema rejects integers less than 2
        let instance = json!(1);
        assert!(validator.validate(&instance).is_err());
        assert!(!validator.is_valid(&instance));

        // Invalid `minimum` value - meta-schema validation catches this first
        let schema = json!({ "minimum": "foo" });
        let error = crate::options()
            .with_keyword("minimum", custom_minimum_factory)
            .build(&schema)
            .expect_err("Should fail");
        // The meta-schema validates before our factory runs, so we get a type error
        assert_eq!(error.to_string(), "\"foo\" is not of type \"number\"");
    }

    #[test]
    fn custom_keyword_validation_error_paths() {
        struct AlwaysFailValidator;
        impl<'i> Keyword<'i> for AlwaysFailValidator {
            fn validate(&self, _instance: &'i Value) -> Result<(), ValidationError<'i>> {
                Err(ValidationError::custom("always fails"))
            }
            fn is_valid(&self, _instance: &'i Value) -> bool {
                false
            }
        }

        fn always_fail_factory<'a>(
            _: &'a Map<String, Value>,
            _: &'a Value,
            _: Location,
        ) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
            Ok(Box::new(AlwaysFailValidator))
        }

        let schema = json!({
            "properties": {
                "name": { "alwaysFail": true }
            }
        });
        let validator = crate::options()
            .with_keyword("alwaysFail", always_fail_factory)
            .build(&schema)
            .expect("Valid schema");

        let instance = json!({"name": "test"});
        let error = validator.validate(&instance).expect_err("Should fail");
        assert_eq!(error.instance_path().as_str(), "/name");
        assert_eq!(error.schema_path().as_str(), "/properties/name/alwaysFail");
        assert_eq!(
            error.evaluation_path().as_str(),
            "/properties/name/alwaysFail"
        );
        assert_eq!(error.kind().keyword(), "alwaysFail");
    }

    #[test]
    fn custom_keyword_factory_error_schema_path() {
        fn failing_factory<'a>(
            _: &'a Map<String, Value>,
            _: &'a Value,
            _: Location,
        ) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
            Err(ValidationError::schema("invalid schema value"))
        }

        let schema = json!({
            "properties": {
                "field": { "myKeyword": "bad-value" }
            }
        });
        let error = crate::options()
            .with_keyword("myKeyword", failing_factory)
            .build(&schema)
            .expect_err("Should fail");
        assert_eq!(error.to_string(), "invalid schema value");
        assert_eq!(error.schema_path().as_str(), "/properties/field/myKeyword");
    }

    #[test]
    fn custom_keyword_forwards_nested_error_kind() {
        struct Delegate(Validator);

        impl<'i> Keyword<'i> for Delegate {
            fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
                self.0.validate(instance)
            }
            fn is_valid(&self, instance: &'i Value) -> bool {
                self.0.is_valid(instance)
            }
        }

        fn delegate<'a>(
            _: &'a Map<String, Value>,
            value: &'a Value,
            _: Location,
        ) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
            let validator = crate::validator_for(value).map_err(ValidationError::to_owned)?;
            Ok(Box::new(Delegate(validator)))
        }

        let validator = crate::options()
            .with_keyword("inner", delegate)
            .build(&json!({"properties": {"a": {"inner": {"minimum": 3}}}}))
            .expect("Valid schema");
        let instance = json!({"a": 1});
        let error = validator.validate(&instance).expect_err("Should fail");
        assert_eq!(
            (
                error.to_string(),
                error.kind().keyword(),
                error.instance_path().as_str(),
                error.schema_path().as_str(),
                error.evaluation_path().as_str(),
            ),
            (
                "1 is less than the minimum of 3".to_string(),
                "minimum",
                "/a",
                "/properties/a/inner",
                "/properties/a/inner",
            )
        );
    }
    struct AcceptAny;

    impl<'i> Keyword<'i> for AcceptAny {
        fn validate(&self, _: &'i Value) -> Result<(), ValidationError<'i>> {
            Ok(())
        }

        fn is_valid(&self, _: &'i Value) -> bool {
            true
        }
    }

    const ARRAY_SHAPE: &str =
        r#"{"type": "array", "minItems": 2, "maxItems": 3, "items": {"type": "number"}}"#;
    const LENGTH_RANGE: &str = r#"{"minLength": 2, "maxLength": 3}"#;
    const PROPERTIES_REQUIRED_2: &str =
        r#"{"properties": {"a": {"type": "integer"}}, "required": ["a", "b"]}"#;
    const PROPERTIES_CLOSED_REQUIRED_1: &str = r#"{"properties": {"a": {"type": "integer"}}, "additionalProperties": false, "required": ["a"]}"#;
    const PROPERTIES_ADDITIONAL_SCHEMA: &str =
        r#"{"properties": {"a": {"type": "integer"}}, "additionalProperties": {"type": "string"}}"#;
    const PATTERN_PROPERTIES_CLOSED: &str =
        r#"{"patternProperties": {"^a": {"type": "integer"}}, "additionalProperties": false}"#;

    #[test_case(ARRAY_SHAPE, "type", &json!("abc"), true; "array shape: custom type replaces the type check")]
    #[test_case(ARRAY_SHAPE, "minItems", &json!([1]), true; "array shape: custom minItems replaces the lower bound")]
    #[test_case(ARRAY_SHAPE, "maxItems", &json!([1, 2, 3, 4]), true; "array shape: custom maxItems replaces the upper bound")]
    #[test_case(ARRAY_SHAPE, "items", &json!(["a", "b"]), true; "array shape: custom items replaces the element check")]
    #[test_case(ARRAY_SHAPE, "items", &json!("abc"), false; "array shape: custom items keeps type")]
    #[test_case(ARRAY_SHAPE, "items", &json!([1]), false; "array shape: custom items keeps minItems")]
    #[test_case(ARRAY_SHAPE, "items", &json!([1, 2, 3, 4]), false; "array shape: custom items keeps maxItems")]
    #[test_case(LENGTH_RANGE, "minLength", &json!("a"), true; "length range: custom minLength replaces the lower bound")]
    #[test_case(LENGTH_RANGE, "minLength", &json!("abcd"), false; "length range: custom minLength keeps maxLength")]
    #[test_case(LENGTH_RANGE, "maxLength", &json!("abcd"), true; "length range: custom maxLength replaces the upper bound")]
    #[test_case(LENGTH_RANGE, "maxLength", &json!("a"), false; "length range: custom maxLength keeps minLength")]
    #[test_case(PROPERTIES_REQUIRED_2, "required", &json!({}), true; "properties with required: custom required replaces the presence check")]
    #[test_case(PROPERTIES_REQUIRED_2, "required", &json!({"a": "x", "b": 1}), false; "properties with required: custom required keeps properties")]
    #[test_case(PROPERTIES_REQUIRED_2, "properties", &json!({"a": "x", "b": 1}), true; "properties with required: custom properties replaces the value check")]
    #[test_case(PROPERTIES_REQUIRED_2, "properties", &json!({}), false; "properties with required: custom properties keeps required")]
    #[test_case(PROPERTIES_CLOSED_REQUIRED_1, "required", &json!({}), true; "closed object: custom required replaces the presence check")]
    #[test_case(PROPERTIES_CLOSED_REQUIRED_1, "additionalProperties", &json!({"a": 1, "z": 2}), true; "closed object: custom additionalProperties accepts extra keys")]
    #[test_case(PROPERTIES_CLOSED_REQUIRED_1, "additionalProperties", &json!({}), false; "closed object: custom additionalProperties keeps required")]
    #[test_case(PROPERTIES_CLOSED_REQUIRED_1, "additionalProperties", &json!({"a": "x"}), false; "closed object: custom additionalProperties keeps properties")]
    #[test_case(PROPERTIES_ADDITIONAL_SCHEMA, "additionalProperties", &json!({"a": "x"}), false; "additionalProperties schema: custom additionalProperties keeps properties")]
    #[test_case(PATTERN_PROPERTIES_CLOSED, "additionalProperties", &json!({"zz": 1}), true; "closed patterns: custom additionalProperties accepts unmatched keys")]
    #[test_case(PATTERN_PROPERTIES_CLOSED, "additionalProperties", &json!({"ab": "x"}), false; "closed patterns: custom additionalProperties keeps patternProperties")]
    fn fused_validators_yield_to_custom_keywords(
        schema: &str,
        keyword: &str,
        instance: &Value,
        expected: bool,
    ) {
        let schema: Value = serde_json::from_str(schema).expect("Invalid JSON");
        let validator = crate::options()
            .with_keyword(keyword, |_, _, _| Ok(Box::new(AcceptAny)))
            .build(&schema)
            .expect("Invalid schema");
        assert_eq!(validator.is_valid(instance), expected);
        assert_eq!(validator.validate(instance).is_ok(), expected);
        assert_eq!(validator.iter_errors(instance).count() == 0, expected);
    }

    #[test]
    fn custom_additional_properties_does_not_duplicate_required() {
        let schema = json!({"properties": {"a": {"type": "integer"}}, "additionalProperties": false, "required": ["a", "b"]});
        let validator = crate::options()
            .with_keyword("additionalProperties", |_, _, _| Ok(Box::new(AcceptAny)))
            .build(&schema)
            .expect("Invalid schema");
        assert_eq!(validator.iter_errors(&json!({})).count(), 2);
    }

    #[test]
    fn test_validator_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Validator>();
    }

    #[test]
    fn test_validator_map_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ValidatorMap>();
    }

    #[test]
    fn test_validator_clone() {
        let schema = json!({"type": "string", "minLength": 3});
        let validator = crate::validator_for(&schema).expect("Valid schema");

        // Clone the validator
        let cloned = validator.clone();

        // Both validators should work independently
        assert!(validator.is_valid(&json!("hello")));
        assert!(!validator.is_valid(&json!("hi")));

        assert!(cloned.is_valid(&json!("hello")));
        assert!(!cloned.is_valid(&json!("hi")));

        // Verify they validate the same way
        assert_eq!(
            validator.is_valid(&json!("test")),
            cloned.is_valid(&json!("test"))
        );
    }

    #[test]
    fn ref_with_required_multiple_missing_clones_deferred_eval_path() {
        // Tests the Deferred branch of LazyEvaluationPath::clone()
        // When $ref is involved and multiple required properties are missing,
        // the eval_path is cloned for each error
        let schema = json!({
            "$defs": {
                "Person": {
                    "type": "object",
                    "required": ["name", "age", "email"]
                }
            },
            "$ref": "#/$defs/Person"
        });
        let validator = crate::validator_for(&schema).expect("Valid schema");
        let instance = json!({});

        let errors: Vec<_> = validator.iter_errors(&instance).collect();

        // Should have 3 errors for the 3 missing required properties
        assert_eq!(errors.len(), 3);

        // All errors should have evaluation paths that include the $ref traversal
        for error in &errors {
            assert_eq!(error.schema_path().as_str(), "/$defs/Person/required");
            assert_eq!(error.evaluation_path().as_str(), "/$ref/required");
        }
    }

    #[test]
    fn test_validator_map_get_existing() {
        let schema = json!({
            "$defs": {
                "User": {"type": "object", "required": ["name"]}
            }
        });
        let map = crate::validator_map_for(&schema).unwrap();
        let v = map.get("#/$defs/User").unwrap();
        assert!(v.is_valid(&json!({"name": "Alice"})));
        assert!(!v.is_valid(&json!({})));
    }

    #[test]
    fn test_validator_map_root_always_present() {
        let schema = json!({"type": "string"});
        let map = crate::validator_map_for(&schema).unwrap();
        assert!(map.get("#").unwrap().is_valid(&json!("hello")));
        assert!(!map.get("#").unwrap().is_valid(&json!(42)));
    }

    #[test]
    fn test_validator_map_get_missing_returns_none() {
        let schema = json!({"type": "object"});
        let map = crate::validator_map_for(&schema).unwrap();
        assert!(map.get("#/nonexistent").is_none());
    }

    #[test]
    fn test_validator_map_contains_key() {
        let schema = json!({"$defs": {"Foo": {"type": "string"}}});
        let map = crate::validator_map_for(&schema).unwrap();
        assert!(map.contains_key("#/$defs/Foo"));
        assert!(!map.contains_key("#/missing"));
    }

    #[test]
    fn test_validator_map_keys_includes_root_and_defs() {
        let schema = json!({
            "type": "object",
            "$defs": {"Foo": {"type": "string"}}
        });
        let map = crate::validator_map_for(&schema).unwrap();
        let keys: std::collections::HashSet<&str> = map.keys().collect();
        assert!(keys.contains("#"));
        assert!(keys.contains("#/$defs/Foo"));
    }

    #[test]
    fn test_validator_map_index() {
        let schema = json!({"$defs": {"Bar": {"type": "integer"}}});
        let map = crate::validator_map_for(&schema).unwrap();
        assert!(map["#/$defs/Bar"].is_valid(&json!(42)));
        assert!(!map["#/$defs/Bar"].is_valid(&json!("nope")));
    }

    #[test]
    #[should_panic(expected = "JSON pointer '#/nonexistent' not found in ValidatorMap")]
    fn test_validator_map_index_panics_on_missing() {
        let schema = json!({"type": "object"});
        let map = crate::validator_map_for(&schema).unwrap();
        let _ = &map["#/nonexistent"];
    }

    #[test]
    fn test_validator_map_via_options() {
        let schema = json!({"$defs": {"S": {"type": "string"}}});
        let map = crate::options().build_map(&schema).unwrap();
        assert!(map.get("#/$defs/S").unwrap().is_valid(&json!("hello")));
    }

    #[cfg(all(feature = "resolve-async", not(target_family = "wasm")))]
    #[tokio::test]
    async fn test_async_validator_map_for() {
        let schema = json!({"$defs": {"T": {"type": "string"}}});
        let map = crate::async_validator_map_for(&schema).await.unwrap();
        assert!(map["#/$defs/T"].is_valid(&json!("hello")));
        assert!(!map["#/$defs/T"].is_valid(&json!(42)));
    }

    #[cfg(all(feature = "resolve-async", not(target_family = "wasm")))]
    #[tokio::test]
    async fn test_async_build_map_via_options() {
        let schema = json!({"$defs": {"N": {"type": "integer"}}});
        let map = crate::async_options().build_map(&schema).await.unwrap();
        assert!(map["#/$defs/N"].is_valid(&json!(7)));
        assert!(!map["#/$defs/N"].is_valid(&json!("not an int")));
    }

    #[test]
    fn test_validator_map_len() {
        let schema = json!({
            "$defs": {
                "A": {"type": "string"},
                "B": {"type": "integer"}
            }
        });
        let map = crate::validator_map_for(&schema).unwrap();
        assert!(map.len() >= 3); // root + A + B
    }

    #[test]
    fn test_validator_map_is_empty_false() {
        let schema = json!({"type": "string"});
        let map = crate::validator_map_for(&schema).unwrap();
        assert!(!map.is_empty());
    }

    #[test]
    fn test_validator_map_nested_pointer() {
        let schema = json!({
            "properties": {
                "name": {"type": "string", "minLength": 2}
            }
        });
        let map = crate::validator_map_for(&schema).unwrap();
        assert!(map["#/properties/name"].is_valid(&json!("Al")));
        assert!(!map["#/properties/name"].is_valid(&json!("A")));
    }

    #[test]
    fn test_validator_map_with_registry() {
        let address_schema = json!({
            "type": "object",
            "properties": {
                "city": {"type": "string"},
                "zip": {"type": "string"}
            },
            "required": ["city"]
        });
        let registry = crate::Registry::new()
            .add("https://example.com/address.json", &address_schema)
            .expect("valid resource")
            .prepare()
            .expect("registry build failed");

        let schema = json!({
            "$defs": {
                "Address": {"$ref": "https://example.com/address.json"}
            }
        });
        let map = crate::options()
            .with_registry(&registry)
            .build_map(&schema)
            .unwrap();

        let v = map.get("#/$defs/Address").unwrap();
        assert!(v.is_valid(&json!({"city": "NYC", "zip": "10001"})));
        assert!(!v.is_valid(&json!({"zip": "10001"}))); // missing required "city"
    }

    #[test]
    fn test_validator_map_with_registry_multiple_refs() {
        let user_schema = json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"]
        });
        let role_schema = json!({
            "type": "string",
            "enum": ["admin", "user", "guest"]
        });
        let registry = crate::Registry::new()
            .add("https://example.com/user.json", &user_schema)
            .expect("valid resource")
            .add("https://example.com/role.json", &role_schema)
            .expect("valid resource")
            .prepare()
            .expect("registry build failed");

        let schema = json!({
            "$defs": {
                "User": {"$ref": "https://example.com/user.json"},
                "Role": {"$ref": "https://example.com/role.json"}
            }
        });
        let map = crate::options()
            .with_registry(&registry)
            .build_map(&schema)
            .unwrap();

        assert!(map["#/$defs/User"].is_valid(&json!({"name": "Alice"})));
        assert!(!map["#/$defs/User"].is_valid(&json!({})));
        assert!(map["#/$defs/Role"].is_valid(&json!("admin")));
        assert!(!map["#/$defs/Role"].is_valid(&json!("superuser")));
    }

    #[cfg(all(feature = "resolve-async", not(target_family = "wasm")))]
    #[tokio::test]
    async fn test_async_validator_map_with_registry() {
        let address_schema = json!({
            "type": "object",
            "properties": {"city": {"type": "string"}},
            "required": ["city"]
        });
        let registry = crate::Registry::new()
            .add("https://example.com/address.json", &address_schema)
            .expect("valid resource")
            .prepare()
            .expect("registry build failed");

        let schema = json!({
            "$defs": {
                "Address": {"$ref": "https://example.com/address.json"}
            }
        });
        let map = crate::async_options()
            .with_registry(&registry)
            .build_map(&schema)
            .await
            .unwrap();

        let v = map.get("#/$defs/Address").unwrap();
        assert!(v.is_valid(&json!({"city": "NYC"})));
        assert!(!v.is_valid(&json!({})));
    }

    #[cfg(all(feature = "resolve-async", not(target_family = "wasm")))]
    #[tokio::test]
    async fn test_async_validator_map_with_registry_multiple_refs() {
        let user_schema = json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"]
        });
        let role_schema = json!({
            "type": "string",
            "enum": ["admin", "user", "guest"]
        });
        let registry = crate::Registry::new()
            .add("https://example.com/user.json", &user_schema)
            .expect("valid resource")
            .add("https://example.com/role.json", &role_schema)
            .expect("valid resource")
            .prepare()
            .expect("registry build failed");

        let schema = json!({
            "$defs": {
                "User": {"$ref": "https://example.com/user.json"},
                "Role": {"$ref": "https://example.com/role.json"}
            }
        });
        let map = crate::async_options()
            .with_registry(&registry)
            .build_map(&schema)
            .await
            .unwrap();

        assert!(map["#/$defs/User"].is_valid(&json!({"name": "Alice"})));
        assert!(!map["#/$defs/User"].is_valid(&json!({})));
        assert!(map["#/$defs/Role"].is_valid(&json!("admin")));
        assert!(!map["#/$defs/Role"].is_valid(&json!("superuser")));
    }

    #[test_case(&json!({"minItems": 2}), &json!([1, 2]), &json!({"valid": true, "details": [
        {"valid": true, "evaluationPath": "", "schemaLocation": "", "instanceLocation": ""},
        {"valid": true, "evaluationPath": "/minItems", "schemaLocation": "/minItems", "instanceLocation": ""}
    ]}); "minItems valid")]
    #[test_case(&json!({"minItems": 2}), &json!([1]), &json!({"valid": false, "details": [
        {"valid": false, "evaluationPath": "", "schemaLocation": "", "instanceLocation": ""},
        {"valid": false, "evaluationPath": "/minItems", "schemaLocation": "/minItems", "instanceLocation": "",
         "errors": {"minItems": "[1] has less than 2 items"}}
    ]}); "minItems invalid")]
    #[test_case(&json!({"maxItems": 1}), &json!([1, 2]), &json!({"valid": false, "details": [
        {"valid": false, "evaluationPath": "", "schemaLocation": "", "instanceLocation": ""},
        {"valid": false, "evaluationPath": "/maxItems", "schemaLocation": "/maxItems", "instanceLocation": "",
         "errors": {"maxItems": "[1,2] has more than 1 item"}}
    ]}); "maxItems invalid")]
    #[test_case(&json!({"minLength": 2}), &json!("a"), &json!({"valid": false, "details": [
        {"valid": false, "evaluationPath": "", "schemaLocation": "", "instanceLocation": ""},
        {"valid": false, "evaluationPath": "/minLength", "schemaLocation": "/minLength", "instanceLocation": "",
         "errors": {"minLength": "\"a\" is shorter than 2 characters"}}
    ]}); "minLength invalid")]
    #[test_case(&json!({"maxLength": 1}), &json!("ab"), &json!({"valid": false, "details": [
        {"valid": false, "evaluationPath": "", "schemaLocation": "", "instanceLocation": ""},
        {"valid": false, "evaluationPath": "/maxLength", "schemaLocation": "/maxLength", "instanceLocation": "",
         "errors": {"maxLength": "\"ab\" is longer than 1 character"}}
    ]}); "maxLength invalid")]
    #[test_case(&json!({"minProperties": 1}), &json!({}), &json!({"valid": false, "details": [
        {"valid": false, "evaluationPath": "", "schemaLocation": "", "instanceLocation": ""},
        {"valid": false, "evaluationPath": "/minProperties", "schemaLocation": "/minProperties", "instanceLocation": "",
         "errors": {"minProperties": "{} has less than 1 property"}}
    ]}); "minProperties invalid")]
    #[test_case(&json!({"maxProperties": 0}), &json!({"a": 1}), &json!({"valid": false, "details": [
        {"valid": false, "evaluationPath": "", "schemaLocation": "", "instanceLocation": ""},
        {"valid": false, "evaluationPath": "/maxProperties", "schemaLocation": "/maxProperties", "instanceLocation": "",
         "errors": {"maxProperties": "{\"a\":1} has more than 0 properties"}}
    ]}); "maxProperties invalid")]
    fn size_keyword_output(schema: &Value, instance: &Value, expected: &Value) {
        let validator = crate::validator_for(schema).expect("Valid schema");
        let list = serde_json::to_value(validator.evaluate(instance).list()).expect("List output");
        assert_eq!(&list, expected);
    }
}
