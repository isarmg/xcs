#[cfg(test)]
mod tests {
    use referencing::Draft;
    use serde_json::{json, Value};
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

    // The reference beside the keyword evaluates the items of the node it points to
    #[test_case(&json!({"$schema": "https://json-schema.org/draft/2019-09/schema", "$defs": {"R": {"items": [{"type": "integer"}, {"$recursiveRef": "#", "unevaluatedItems": false}]}}, "$ref": "#/$defs/R"}), &json!([1, [2]]), &[]; "2019-09 prefixed item")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft/2019-09/schema", "$defs": {"R": {"items": [{"type": "integer"}, {"$recursiveRef": "#", "unevaluatedItems": false}]}}, "$ref": "#/$defs/R"}), &json!([1, [2, [3]]]), &[]; "2019-09 nested prefixed item")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft/2019-09/schema", "$defs": {"R": {"items": [{"type": "integer"}, {"$recursiveRef": "#", "unevaluatedItems": false}]}}, "$ref": "#/$defs/R"}), &json!([1, [2, 3, 4]]), &[("/1", "/$ref/items/1/unevaluatedItems", "Unevaluated items are not allowed ('4' was unexpected)")]; "2019-09 item past the prefix")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft/2019-09/schema", "$defs": {"R": {"items": [{"type": "integer"}, {"$recursiveRef": "#", "unevaluatedItems": false}]}}, "$ref": "#/$defs/R"}), &json!([1, [2, [3, 4, 5]]]), &[("/1/1", "/$ref/items/1/$recursiveRef/$ref/items/1/unevaluatedItems", "Unevaluated items are not allowed ('5' was unexpected)")]; "2019-09 nested item past the prefix")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "$defs": {"R": {"prefixItems": [{"type": "integer"}, {"$dynamicRef": "#", "unevaluatedItems": false}]}}, "$ref": "#/$defs/R"}), &json!([1, [2]]), &[]; "2020-12 prefixed item")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "$defs": {"R": {"prefixItems": [{"type": "integer"}, {"$dynamicRef": "#", "unevaluatedItems": false}]}}, "$ref": "#/$defs/R"}), &json!([1, [2, 3, 4]]), &[("/1", "/$ref/prefixItems/1/unevaluatedItems", "Unevaluated items are not allowed ('4' was unexpected)")]; "2020-12 item past the prefix")]
    fn reference_to_root_evaluates_the_root_items(
        schema: &Value,
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
        assert_eq!(errors(schema, instance), expected);
    }

    #[test_case(Draft::Draft201909, &json!([]), true; "2019-09 empty array")]
    #[test_case(Draft::Draft201909, &json!([1]), false; "2019-09 leaves the first item unevaluated")]
    #[test_case(Draft::Draft202012, &json!([1]), true; "2020-12 evaluates the prefixed index")]
    #[test_case(Draft::Draft202012, &json!([1, 2]), false; "2020-12 leaves the index past the prefix unevaluated")]
    fn prefix_items_evaluate_only_where_the_draft_defines_them(
        draft: Draft,
        instance: &Value,
        expected: bool,
    ) {
        let validator = crate::options()
            .with_draft(draft)
            .build(&json!({"prefixItems": [{"type": "integer"}], "unevaluatedItems": false}))
            .expect("schema compiles");
        assert_eq!(validator.is_valid(instance), expected);
    }

    #[test]
    fn dynamic_ref_cycle_does_not_overflow() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root",
            "$dynamicAnchor": "node",
            "type": "array",
            "$dynamicRef": "#node",
            "unevaluatedItems": false
        });

        let validator = crate::options().build(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!([])));
    }

    // A `$ref` cycle back to the node evaluates exactly what the node evaluates
    #[test_case(&json!({"$ref": "#"}); "self reference")]
    #[test_case(&json!({"$id": "https://example.com/root.json", "$ref": "https://example.com/root.json"}); "self through id")]
    #[test_case(&json!({"allOf": [{"$ref": "#"}]}); "self through allOf")]
    #[test_case(&json!({"if": {"$ref": "#"}}); "self through if")]
    #[test_case(&json!({"$defs": {"a": {"$ref": "#/$defs/b"}, "b": {"$ref": "#/$defs/a"}}, "$ref": "#/$defs/a"}); "mutually recursive definitions")]
    #[test_case(&json!({"$defs": {"a": {"$ref": "#/$defs/a"}}, "$ref": "#/$defs/a"}); "self recursive definition")]
    fn ref_cycle_evaluates_what_the_node_evaluates(applicator: &Value) {
        let mut schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "prefixItems": [true],
            "unevaluatedItems": false
        });
        schema
            .as_object_mut()
            .expect("object schema")
            .extend(applicator.as_object().expect("object applicator").clone());

        let validator = crate::validator_for(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!([1])));
        assert!(!validator.is_valid(&json!([1, 2])));
    }

    // The keyword inside a recursive definition sees the definition's own prefix
    #[test]
    fn ref_cycle_inside_definition_evaluates_the_definition() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": {
                "node": {
                    "prefixItems": [{"$ref": "#/$defs/node"}],
                    "allOf": [{"$ref": "#/$defs/node"}],
                    "unevaluatedItems": false
                }
            },
            "$ref": "#/$defs/node"
        });

        let validator = crate::validator_for(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!([[[]]])));
        assert!(!validator.is_valid(&json!([[[], 1]])));
    }

    #[test]
    fn prefix_items_do_not_evaluate_without_applicator_vocabulary() {
        let meta = json!({
            "$id": "json-schema:///meta/no-applicator-items",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/validation": true,
                "https://json-schema.org/draft/2020-12/vocab/unevaluated": true,
                "https://json-schema.org/draft/2020-12/vocab/format-annotation": true
            }
        });
        let registry = crate::Registry::new()
            .add("json-schema:///meta/no-applicator-items", &meta)
            .expect("resource accepted")
            .prepare()
            .expect("registry build failed");
        let schema = json!({
            "$schema": "json-schema:///meta/no-applicator-items",
            "prefixItems": [{"type": "integer"}],
            "unevaluatedItems": false
        });
        let validator = crate::options()
            .with_registry(&registry)
            .build(&schema)
            .expect("schema compiles");
        assert!(validator.is_valid(&json!([])));
        assert!(!validator.is_valid(&json!([1])));
    }

    #[test]
    fn dynamic_ref_cycle_via_all_of_does_not_overflow() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root",
            "$dynamicAnchor": "node",
            "type": "array",
            "allOf": [{ "$dynamicRef": "#node" }],
            "unevaluatedItems": false
        });

        let validator = crate::options().build(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!([])));
    }

    #[test]
    fn test_unevaluated_items_with_recursion() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "allOf": [
                {
                    "$ref": "#/$defs/array_1"
                }
            ],
            "unevaluatedItems": false,
            "$defs": {
                "array_1": {
                    "type": "array",
                    "prefixItems": [
                        {
                            "type": "string"
                        },
                        {
                            "allOf": [
                                {
                                    "$ref": "#/$defs/array_2"
                                }
                            ],
                            "type": "array",
                            "unevaluatedItems": false
                        }
                    ]
                },
                "array_2": {
                    "type": "array",
                    "prefixItems": [
                        {
                            "type": "number"
                        },
                        {
                            "allOf": [
                                {
                                    "$ref": "#/$defs/array_1"
                                }
                            ],
                            "type": "array",
                            "unevaluatedItems": false
                        }
                    ]
                }
            }
        });

        let validator = crate::validator_for(&schema).expect("Schema should compile");

        // This instance should fail validation because the nested array has an unevaluated item
        let instance = json!([
            "string",
            [
                42,
                [
                    "string",
                    [
                        42,
                        "unexpected" // This item should cause validation to fail
                    ]
                ]
            ]
        ]);

        assert!(!validator.is_valid(&instance));
        assert!(validator.validate(&instance).is_err());

        // This instance should pass validation as all items are evaluated
        let valid_instance = json!(["string", [42, ["string", [42]]]]);

        assert!(validator.is_valid(&valid_instance));
        assert!(validator.validate(&valid_instance).is_ok());
    }

    #[test_case(&json!({"allOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "allOf")]
    #[test_case(&json!({"anyOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "anyOf")]
    #[test_case(&json!({"oneOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "oneOf")]
    #[test_case(&json!({"if": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "if branch")]
    #[test_case(&json!({"if": {}, "then": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "then branch")]
    #[test_case(&json!({"if": {"not": {}}, "else": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "else branch")]
    fn subresource_id_resolves_relative_reference_against_its_own_base(applicator: &Value) {
        let mut schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root.json",
            "$defs": {
                "reachable_through_the_branch_id": {
                    "$id": "https://example.com/nested/target.json",
                    "prefixItems": [{"type": "string"}]
                },
                "reachable_through_the_root_id": {
                    "$id": "https://example.com/target.json",
                    "prefixItems": [{"type": "string"}, {"type": "string"}]
                }
            },
            "unevaluatedItems": false
        });
        schema
            .as_object_mut()
            .expect("object schema")
            .extend(applicator.as_object().expect("object applicator").clone());

        let validator = crate::validator_for(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!(["x"])));
        assert!(!validator.is_valid(&json!(["x", "y"])));
    }

    #[test_case(&json!({"allOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "allOf")]
    #[test_case(&json!({"anyOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "anyOf")]
    #[test_case(&json!({"oneOf": [{"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}]}); "oneOf")]
    #[test_case(&json!({"if": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "if branch")]
    #[test_case(&json!({"if": {}, "then": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "then branch")]
    #[test_case(&json!({"if": {"not": {}}, "else": {"$id": "https://example.com/nested/branch.json", "$ref": "target.json"}}); "else branch")]
    fn subresource_id_keeps_the_schema_compilable(applicator: &Value) {
        let mut schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root.json",
            "$defs": {
                "target": {
                    "$id": "https://example.com/nested/target.json",
                    "prefixItems": [{"type": "string"}]
                }
            },
            "unevaluatedItems": false
        });
        schema
            .as_object_mut()
            .expect("object schema")
            .extend(applicator.as_object().expect("object applicator").clone());

        crate::validator_for(&schema).expect("schema compiles");
    }

    #[test]
    fn reference_target_resolves_its_own_relative_reference() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root.json",
            "$defs": {
                "outer": {
                    "$id": "https://example.com/nested/outer.json",
                    "$ref": "inner.json"
                },
                "reachable_through_the_outer_id": {
                    "$id": "https://example.com/nested/inner.json",
                    "prefixItems": [{"type": "string"}]
                },
                "reachable_through_the_root_id": {
                    "$id": "https://example.com/inner.json",
                    "prefixItems": [{"type": "string"}, {"type": "string"}]
                }
            },
            "$ref": "https://example.com/nested/outer.json",
            "unevaluatedItems": false
        });

        let validator = crate::validator_for(&schema).expect("schema compiles");

        assert!(validator.is_valid(&json!(["x"])));
        assert!(!validator.is_valid(&json!(["x", "y"])));
    }

    const FIRST_UNEXPECTED: &[(&str, &str)] = &[(
        "/unevaluatedItems",
        "Unevaluated items are not allowed ('1' was unexpected)",
    )];

    // Boolean and empty subschemas evaluate nothing; the branches they select still do
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"if": true, "then": {"items": [{}]}}), &[]; "2019-09 if true with then")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"if": false, "else": {"items": [{}]}}), &[]; "2019-09 if false with else")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"if": true, "else": {"items": [{}]}}), FIRST_UNEXPECTED; "2019-09 if true with else only")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"if": false, "then": {"items": [{}]}}), FIRST_UNEXPECTED; "2019-09 if false with then only")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"if": true, "then": true}), FIRST_UNEXPECTED; "2019-09 if true with then true")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"if": true, "then": {"items": true}}), &[]; "2019-09 if true with then items true")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"if": {}, "then": {"items": [{}]}}), &[]; "2019-09 if empty with then")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"if": true, "then": {"if": false, "else": {"items": [{}]}}}), &[]; "2019-09 nested boolean if")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"allOf": [true]}), FIRST_UNEXPECTED; "2019-09 allOf true")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"allOf": [true, {"items": [{}]}]}), &[]; "2019-09 allOf true and object")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"anyOf": [true, {"items": [{}]}]}), &[]; "2019-09 anyOf true and object")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"oneOf": [false, {"items": [{}]}]}), &[]; "2019-09 oneOf false and object")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"not": false}), FIRST_UNEXPECTED; "2019-09 not false")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"$defs": {"t": true}, "$ref": "#/$defs/t"}), FIRST_UNEXPECTED; "2019-09 ref to true")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"items": true}), &[]; "2019-09 items true")]
    #[test_case("https://json-schema.org/draft/2019-09/schema", &json!({"items": [true]}), &[]; "2019-09 items true")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"if": true, "then": {"prefixItems": [{}]}}), &[]; "2020-12 if true with then")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"if": false, "else": {"prefixItems": [{}]}}), &[]; "2020-12 if false with else")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"if": true, "else": {"prefixItems": [{}]}}), FIRST_UNEXPECTED; "2020-12 if true with else only")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"if": false, "then": {"prefixItems": [{}]}}), FIRST_UNEXPECTED; "2020-12 if false with then only")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"if": true, "then": true}), FIRST_UNEXPECTED; "2020-12 if true with then true")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"if": true, "then": {"items": true}}), &[]; "2020-12 if true with then items true")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"if": {}, "then": {"prefixItems": [{}]}}), &[]; "2020-12 if empty with then")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"if": true, "then": {"if": false, "else": {"prefixItems": [{}]}}}), &[]; "2020-12 nested boolean if")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"allOf": [true]}), FIRST_UNEXPECTED; "2020-12 allOf true")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"allOf": [true, {"prefixItems": [{}]}]}), &[]; "2020-12 allOf true and object")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"anyOf": [true, {"prefixItems": [{}]}]}), &[]; "2020-12 anyOf true and object")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"oneOf": [false, {"prefixItems": [{}]}]}), &[]; "2020-12 oneOf false and object")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"not": false}), FIRST_UNEXPECTED; "2020-12 not false")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"$defs": {"t": true}, "$ref": "#/$defs/t"}), FIRST_UNEXPECTED; "2020-12 ref to true")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"items": true}), &[]; "2020-12 items true")]
    #[test_case("https://json-schema.org/draft/2020-12/schema", &json!({"prefixItems": [true]}), &[]; "2020-12 prefixItems true")]
    fn boolean_subschemas(draft: &str, applicator: &Value, expected: &[(&str, &str)]) {
        let expected: Vec<_> = expected
            .iter()
            .map(|(location, message)| ((*location).to_owned(), (*message).to_owned()))
            .collect();
        // `evaluate()` also lists the `false` subschema rejecting the unevaluated value
        let mut evaluation_expected = expected.clone();
        if !expected.is_empty() {
            evaluation_expected.push((
                "/unevaluatedItems".to_owned(),
                "False schema does not allow 1".to_owned(),
            ));
        }
        let mut schema = json!({"$schema": draft, "unevaluatedItems": false});
        schema
            .as_object_mut()
            .expect("object schema")
            .extend(applicator.as_object().expect("object applicator").clone());
        assert_eq!(
            errors_and_evaluation(&schema, &json!([1])),
            (
                expected.is_empty(),
                expected.clone(),
                expected.is_empty(),
                evaluation_expected
            )
        );
    }

    #[test]
    fn malformed() {
        crate::tests_util::assert_compile_error(
            &json!({"unevaluatedItems": 5}),
            "5 is not of types \"boolean\", \"object\"",
            "/unevaluatedItems",
        );
    }
}
