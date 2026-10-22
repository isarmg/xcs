#[cfg(test)]
mod tests {
    use crate::error::ValidationErrorKind;
    use json_codec::{json, Value};
    use test_case::test_case;

    fn errors(schema: &Value, instance: &Value) -> Vec<(String, String, String)> {
        let validator = crate::validator_for(schema).expect("schema compiles");
        validator
            .iter_errors(instance)
            .map(|error| {
                (
                    error.instance_path().as_str().to_owned(),
                    error.evaluation_path().as_str().to_owned(),
                    error.to_string(),
                )
            })
            .collect()
    }

    /// `is_valid`, `iter_errors`, `evaluate()` validity and `evaluate()` errors, each error as
    /// a (schema location, message) pair.
    type Outcome = (bool, Vec<(String, String)>, bool, Vec<(String, String)>);

    fn errors_and_evaluation(schema: &Value, instance: &Value) -> Outcome {
        let validator = crate::validator_for(schema).expect("schema compiles");
        let errors = validator
            .iter_errors(instance)
            .map(|error| {
                (
                    error.evaluation_path().as_str().to_owned(),
                    error.to_string(),
                )
            })
            .collect();
        let evaluation = validator.evaluate(instance);
        let evaluation_errors = evaluation
            .iter_errors()
            .map(|entry| (entry.schema_location.to_owned(), entry.error.to_string()))
            .collect();
        (
            validator.is_valid(instance),
            errors,
            evaluation.flag().valid,
            evaluation_errors,
        )
    }

    const A_UNEXPECTED: &[(&str, &str)] = &[(
        "/unevaluatedProperties",
        "Unevaluated properties are not allowed ('a' was unexpected)",
    )];

    // Boolean and empty subschemas evaluate nothing; the branches they select still do
    #[test_case(&json!({"if": true, "then": {"properties": {"a": {}}}}), &[]; "if true with then")]
    #[test_case(&json!({"if": false, "else": {"properties": {"a": {}}}}), &[]; "if false with else")]
    #[test_case(&json!({"if": true, "else": {"properties": {"a": {}}}}), A_UNEXPECTED; "if true with else only")]
    #[test_case(&json!({"if": false, "then": {"properties": {"a": {}}}}), A_UNEXPECTED; "if false with then only")]
    #[test_case(&json!({"if": true, "then": true}), A_UNEXPECTED; "if true with then true")]
    #[test_case(&json!({"if": {}, "then": {"properties": {"a": {}}}}), &[]; "if empty with then")]
    #[test_case(&json!({"if": true, "then": {"if": false, "else": {"properties": {"a": {}}}}}), &[]; "nested boolean if")]
    #[test_case(&json!({"$defs": {"d": {"properties": {"a": {}}}}, "if": true, "then": {"$ref": "#/$defs/d"}}), &[]; "if true with then ref")]
    #[test_case(&json!({"allOf": [true]}), A_UNEXPECTED; "allOf true")]
    #[test_case(&json!({"allOf": [true, {"properties": {"a": {}}}]}), &[]; "allOf true and object")]
    #[test_case(&json!({"anyOf": [true, {"properties": {"a": {}}}]}), &[]; "anyOf true and object")]
    #[test_case(&json!({"oneOf": [true, {"required": ["b"], "properties": {"a": {}}}]}), A_UNEXPECTED; "oneOf true and failing object")]
    #[test_case(&json!({"oneOf": [false, {"properties": {"a": {}}}]}), &[]; "oneOf false and object")]
    #[test_case(&json!({"dependentSchemas": {"a": true}}), A_UNEXPECTED; "dependentSchemas true")]
    #[test_case(&json!({"dependentSchemas": {"a": {"properties": {"a": {}}}, "b": true}}), &[]; "dependentSchemas object and true")]
    #[test_case(&json!({"not": false}), A_UNEXPECTED; "not false")]
    #[test_case(&json!({"$defs": {"t": true}, "$ref": "#/$defs/t"}), A_UNEXPECTED; "ref to true")]
    #[test_case(&json!({"properties": {"a": true}}), &[]; "properties true")]
    #[test_case(&json!({"patternProperties": {"^a": true}}), &[]; "patternProperties true")]
    #[test_case(&json!({"additionalProperties": true}), &[]; "additionalProperties true")]
    fn boolean_subschemas(applicator: &Value, expected: &[(&str, &str)]) {
        let expected: Vec<_> = expected
            .iter()
            .map(|(location, message)| ((*location).to_owned(), (*message).to_owned()))
            .collect();
        // `evaluate()` also lists the `false` subschema rejecting the unevaluated value
        let mut evaluation_expected = expected.clone();
        if !expected.is_empty() {
            evaluation_expected.push((
                "/unevaluatedProperties".to_owned(),
                "False schema does not allow 1".to_owned(),
            ));
        }
        for draft in [
            "https://json-schema.org/draft/2019-09/schema",
            "https://json-schema.org/draft/2020-12/schema",
        ] {
            let mut schema = json!({"$schema": draft, "unevaluatedProperties": false});
            schema
                .as_object_mut()
                .expect("object schema")
                .extend(applicator.as_object().expect("object applicator").clone());
            assert_eq!(
                errors_and_evaluation(&schema, &json!({"a": 1})),
                (
                    expected.is_empty(),
                    expected.clone(),
                    expected.is_empty(),
                    evaluation_expected.clone()
                ),
                "{draft}"
            );
        }
    }

    fn recursive_child(draft: &str, reference: &str) -> Value {
        json!({
            "$schema": draft,
            "$defs": {
                "R": {
                    "properties": {
                        "foo": {"type": "integer"},
                        "child": {reference: "#", "unevaluatedProperties": false}
                    }
                }
            },
            "$ref": "#/$defs/R"
        })
    }

    // The reference beside the keyword evaluates the properties of the node it points to
    #[test_case("https://json-schema.org/draft/2019-09/schema", "$recursiveRef", &json!({"child": {"foo": 1}}), &[]; "2019-09 declared property")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", "$recursiveRef", &json!({"child": {"child": {"foo": 1}}}), &[]; "2019-09 nested declared property")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", "$recursiveRef", &json!({"child": {"bar": 1}}), &[("/child", "/$ref/properties/child/unevaluatedProperties", "Unevaluated properties are not allowed ('bar' was unexpected)")]; "2019-09 undeclared property")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", "$recursiveRef", &json!({"child": {"child": {"bar": 1}}}), &[("/child/child", "/$ref/properties/child/$recursiveRef/$ref/properties/child/unevaluatedProperties", "Unevaluated properties are not allowed ('bar' was unexpected)")]; "2019-09 nested undeclared property")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", "$recursiveRef", &json!({"child": {"foo": "x"}}), &[("/child/foo", "/$ref/properties/child/$recursiveRef/$ref/properties/foo/type", r#""x" is not of type "integer""#)]; "2019-09 declared property of the wrong type")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", "$dynamicRef", &json!({"child": {"foo": 1}}), &[]; "2020-12 declared property")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", "$dynamicRef", &json!({"child": {"bar": 1}}), &[("/child", "/$ref/properties/child/unevaluatedProperties", "Unevaluated properties are not allowed ('bar' was unexpected)")]; "2020-12 undeclared property")]
    fn reference_to_root_evaluates_the_root_properties(
        draft: &str,
        reference: &str,
        instance: &Value,
        expected: &[(&str, &str, &str)],
    ) {
        let expected: Vec<_> = expected
            .iter()
            .map(|(instance_path, evaluation_path, message)| {
                (
                    (*instance_path).to_owned(),
                    (*evaluation_path).to_owned(),
                    (*message).to_owned(),
                )
            })
            .collect();
        assert_eq!(
            errors(&recursive_child(draft, reference), instance),
            expected
        );
    }

    #[test]
    fn evaluated_keys_across_ref_use_target_applicator_vocabulary() {
        let meta = json!({
            "$id": "json-schema:///meta/no-applicator",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/applicator": false,
                "https://json-schema.org/draft/2020-12/vocab/validation": true,
                "https://json-schema.org/draft/2020-12/vocab/unevaluated": true
            }
        });
        let target = json!({
            "$id": "https://example.com/t",
            "$schema": "json-schema:///meta/no-applicator",
            "$defs": {"x": {"properties": {"a": {}}}}
        });
        let root = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "allOf": [{"$ref": "https://example.com/t#/$defs/x"}],
            "unevaluatedProperties": false
        });
        let registry = crate::Registry::new()
            .add("json-schema:///meta/no-applicator", &meta)
            .unwrap()
            .add("https://example.com/t", &target)
            .unwrap()
            .prepare()
            .unwrap();
        let validator = crate::options()
            .with_registry(&registry)
            .build(&root)
            .unwrap();
        assert!(!validator.is_valid(&json!({"a": 1})));
    }

    #[test]
    fn dynamic_ref_cycle_does_not_overflow() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root",
            "$dynamicAnchor": "node",
            "type": "object",
            "$dynamicRef": "#node",
            "unevaluatedProperties": false
        });

        let validator = crate::options().build(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!({})));
    }

    // A `$ref` cycle back to the node evaluates exactly what the node evaluates
    #[test_case(&json!({"$ref": "#"}); "self reference")]
    #[test_case(&json!({"$id": "https://example.com/root.json", "$ref": "https://example.com/root.json"}); "self through id")]
    #[test_case(&json!({"allOf": [{"$ref": "#"}]}); "self through allOf")]
    #[test_case(&json!({"if": {"$ref": "#"}}); "self through if")]
    #[test_case(&json!({"dependentSchemas": {"a": {"$ref": "#"}}}); "self through dependentSchemas")]
    #[test_case(&json!({"$defs": {"a": {"$ref": "#/$defs/b"}, "b": {"$ref": "#/$defs/a"}}, "$ref": "#/$defs/a"}); "mutually recursive definitions")]
    #[test_case(&json!({"$defs": {"a": {"$ref": "#/$defs/a"}}, "$ref": "#/$defs/a"}); "self recursive definition")]
    fn ref_cycle_evaluates_what_the_node_evaluates(applicator: &Value) {
        let mut schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "properties": {"a": true},
            "unevaluatedProperties": false
        });
        schema
            .as_object_mut()
            .expect("object schema")
            .extend(applicator.as_object().expect("object applicator").clone());

        let validator = crate::validator_for(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!({"a": 1})));
        assert!(!validator.is_valid(&json!({"b": 1})));
    }

    // The keyword inside a recursive definition sees the definition's own properties
    #[test]
    fn ref_cycle_inside_definition_evaluates_the_definition() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": {
                "node": {
                    "properties": {"child": {"$ref": "#/$defs/node"}},
                    "allOf": [{"$ref": "#/$defs/node"}],
                    "unevaluatedProperties": false
                }
            },
            "$ref": "#/$defs/node"
        });

        let validator = crate::validator_for(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!({"child": {"child": {}}})));
        assert!(!validator.is_valid(&json!({"child": {"other": 1}})));
    }

    #[test]
    fn properties_do_not_evaluate_without_applicator_vocabulary() {
        let meta = json!({
            "$id": "json-schema:///meta/no-applicator",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/validation": true,
                "https://json-schema.org/draft/2020-12/vocab/unevaluated": true,
                "https://json-schema.org/draft/2020-12/vocab/format-annotation": true
            }
        });
        let registry = crate::Registry::new()
            .add("json-schema:///meta/no-applicator", &meta)
            .expect("resource accepted")
            .prepare()
            .expect("registry build failed");
        let schema = json!({
            "$schema": "json-schema:///meta/no-applicator",
            "properties": {"a": {"type": "integer"}},
            "unevaluatedProperties": false
        });
        let validator = crate::options()
            .with_registry(&registry)
            .build(&schema)
            .expect("schema compiles");
        assert!(validator.is_valid(&json!({})));
        assert!(!validator.is_valid(&json!({"a": 1})));
        assert!(!validator.is_valid(&json!({"b": 2})));
    }

    #[test]
    fn dynamic_ref_cycle_via_all_of_does_not_overflow() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root",
            "$dynamicAnchor": "node",
            "type": "object",
            "allOf": [{ "$dynamicRef": "#node" }],
            "unevaluatedProperties": false
        });

        let validator = crate::options().build(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!({})));
    }

    #[test]
    fn recursive_ref_preserves_unevaluated_properties() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "$id": "https://example.com/root",
            "$recursiveAnchor": true,
            "type": "object",
            "properties": {
                "child": {
                    "type": "object",
                    "properties": {
                        "child": { "$recursiveRef": "#" }
                    },
                    "unevaluatedProperties": false
                }
            },
            "unevaluatedProperties": false
        });

        let validator = crate::options().build(&schema).expect("schema compiles");

        let valid = json!({"child": {"child": {}}});
        assert!(
            validator.is_valid(&valid),
            "expected recursive schema without extras to be valid"
        );

        let invalid = json!({"child": {"child": {"unexpected": 1}}});
        assert!(
            !validator.is_valid(&invalid),
            "unexpected properties should be rejected"
        );

        let errors: Vec<_> = validator.iter_errors(&invalid).collect();
        assert!(
            errors.iter().any(|err| matches!(
                err.kind(),
                ValidationErrorKind::UnevaluatedProperties { .. }
            )),
            "expected unevaluatedProperties error, got {errors:?}"
        );
    }

    #[test_case(&json!({"allOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "allOf")]
    #[test_case(&json!({"anyOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "anyOf")]
    #[test_case(&json!({"oneOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "oneOf")]
    #[test_case(&json!({"if": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "if branch")]
    #[test_case(&json!({"if": {}, "then": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "then branch")]
    #[test_case(&json!({"if": {"not": {}}, "else": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "else branch")]
    #[test_case(&json!({"dependentSchemas": {"a": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}}); "dependentSchemas")]
    fn subresource_id_resolves_relative_reference_against_its_own_base(applicator: &Value) {
        let mut schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root.json",
            "$defs": {
                "reachable_through_the_branch_id": {
                    "$id": "https://example.com/nested/target.json",
                    "properties": {"a": {"type": "string"}}
                },
                "reachable_through_the_root_id": {
                    "$id": "https://example.com/target.json",
                    "properties": {"b": {"type": "integer"}}
                }
            },
            "unevaluatedProperties": false
        });
        schema
            .as_object_mut()
            .expect("object schema")
            .extend(applicator.as_object().expect("object applicator").clone());

        let validator = crate::validator_for(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!({"a": "x"})));
        assert!(!validator.is_valid(&json!({"b": 1})));
    }

    #[test_case(&json!({"allOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "allOf")]
    #[test_case(&json!({"anyOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "anyOf")]
    #[test_case(&json!({"oneOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "oneOf")]
    #[test_case(&json!({"if": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "if branch")]
    #[test_case(&json!({"if": {}, "then": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "then branch")]
    #[test_case(&json!({"if": {"not": {}}, "else": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "else branch")]
    #[test_case(&json!({"dependentSchemas": {"a": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}}); "dependentSchemas")]
    fn subresource_id_keeps_the_schema_compilable(applicator: &Value) {
        let mut schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root.json",
            "$defs": {
                "target": {
                    "$id": "https://example.com/nested/target.json",
                    "properties": {"a": {"type": "string"}}
                }
            },
            "unevaluatedProperties": false
        });
        schema
            .as_object_mut()
            .expect("object schema")
            .extend(applicator.as_object().expect("object applicator").clone());

        crate::validator_for(&schema).expect("schema compiles");
    }

    fn messages(validator: &crate::Validator, instance: &Value) -> Vec<String> {
        validator
            .iter_errors(instance)
            .map(|error| error.to_string())
            .collect()
    }

    fn beside_unevaluated(wrapping: &str, pattern_properties: &Value) -> Value {
        let inner = json!({"patternProperties": pattern_properties});
        match wrapping {
            "direct" => {
                json!({"patternProperties": pattern_properties, "unevaluatedProperties": false})
            }
            "allOf" => json!({"allOf": [inner], "unevaluatedProperties": false}),
            "$ref" => {
                json!({"$defs": {"t": inner}, "$ref": "#/$defs/t", "unevaluatedProperties": false})
            }
            "then" => json!({"if": {}, "then": inner, "unevaluatedProperties": false}),
            _ => unreachable!("unknown wrapping"),
        }
    }

    // Exceeds the engines' default size limit
    const LARGE_PATTERN: &str = r"^\p{L}{300}$";

    #[test_case(true, "direct")]
    #[test_case(true, "allOf")]
    #[test_case(true, "$ref")]
    #[test_case(true, "then")]
    #[test_case(false, "direct")]
    #[test_case(false, "allOf")]
    #[test_case(false, "$ref")]
    #[test_case(false, "then")]
    fn pattern_properties_use_the_configured_size_limit(fancy: bool, wrapping: &str) {
        let schema = beside_unevaluated(wrapping, &json!({LARGE_PATTERN: {"type": "integer"}}));
        let options = if fancy {
            crate::options()
                .with_pattern_options(crate::PatternOptions::fancy_regex().size_limit(1 << 30))
        } else {
            crate::options()
                .with_pattern_options(crate::PatternOptions::regex().size_limit(1 << 30))
        };
        let validator = options.build(&schema).expect("schema compiles");

        let matched = json!({"a".repeat(300): 1});
        let unmatched = json!({"b": 1});
        assert_eq!(
            (
                validator.is_valid(&matched),
                messages(&validator, &matched),
                validator.is_valid(&unmatched),
                messages(&validator, &unmatched),
            ),
            (
                true,
                Vec::<String>::new(),
                false,
                vec!["Unevaluated properties are not allowed ('b' was unexpected)".to_owned()],
            )
        );
    }

    // `patternProperties` does not evaluate a key whose match exceeds the backtrack limit
    #[test_case("direct")]
    #[test_case("allOf")]
    #[test_case("$ref")]
    #[test_case("then")]
    fn pattern_properties_use_the_configured_backtrack_limit(wrapping: &str) {
        let schema = beside_unevaluated(wrapping, &json!({"(?<=ab)c": {"type": "integer"}}));
        let validator = crate::options()
            .with_pattern_options(crate::PatternOptions::fancy_regex().backtrack_limit(1))
            .build(&schema)
            .expect("schema compiles");

        let instance = json!({"abc": "x"});
        assert_eq!(
            (
                validator.is_valid(&instance),
                messages(&validator, &instance)
            ),
            (
                false,
                vec!["Unevaluated properties are not allowed ('abc' was unexpected)".to_owned()],
            )
        );
    }

    #[test]
    fn malformed() {
        crate::tests_util::assert_compile_error(
            &json!({"unevaluatedProperties": 5}),
            "5 is not of types \"boolean\", \"object\"",
            "/unevaluatedProperties",
        );
    }
}
