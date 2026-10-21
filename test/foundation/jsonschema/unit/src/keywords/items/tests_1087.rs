#[cfg(test)]
mod tests {
    use crate::tests_util;
    use referencing::Draft;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(Draft::Draft201909, &json!([]), true; "2019-09 empty array")]
    #[test_case(Draft::Draft201909, &json!([1]), false; "2019-09 items covers the whole array")]
    #[test_case(Draft::Draft202012, &json!([1]), true; "2020-12 items skips the prefix")]
    #[test_case(Draft::Draft202012, &json!([1, 2]), false; "2020-12 items covers past the prefix")]
    fn items_skips_a_prefix_only_where_the_draft_defines_prefix_items(
        draft: Draft,
        instance: &Value,
        expected: bool,
    ) {
        let validator = crate::options()
            .with_draft(draft)
            .build(&json!({"prefixItems": [{"type": "integer"}], "items": false}))
            .expect("schema compiles");
        assert_eq!(validator.is_valid(instance), expected);
    }

    #[test_case(&json!({"items": false}), &json!([1]), "/items")]
    #[test_case(&json!({"items": {"type": "string"}}), &json!([1]), "/items/type")]
    #[test_case(&json!({"prefixItems": [{"type": "string"}]}), &json!([1]), "/prefixItems/0/type")]
    fn schema_location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    #[test_case(&json!({"items": {"type": "string"}}), &json!([1]), "/0"; "string first")]
    #[test_case(&json!({"items": {"type": "string"}}), &json!(["a", 1]), "/1"; "string second")]
    #[test_case(&json!({"items": {"type": "number"}}), &json!(["x"]), "/0"; "number first")]
    #[test_case(&json!({"items": {"type": "integer"}}), &json!([1.5]), "/0"; "integer first")]
    #[test_case(&json!({"items": {"type": "boolean"}}), &json!([1]), "/0"; "boolean first")]
    fn instance_location(schema: &Value, instance: &Value, expected: &str) {
        let validator = crate::validator_for(schema).unwrap();
        let error = validator.iter_errors(instance).next().unwrap();
        assert_eq!(error.instance_path().as_str(), expected);
    }

    fn with_id(draft: Draft, mut schema: Value) -> Value {
        let key = if draft == Draft::Draft4 { "id" } else { "$id" };
        schema[key] = json!("https://example.com/root");
        schema
    }

    fn error_report(
        schema: &Value,
        draft: Draft,
        instance: &Value,
    ) -> Vec<(String, String, String, String)> {
        let validator = crate::options()
            .with_draft(draft)
            .build(schema)
            .expect("schema compiles");
        validator
            .iter_errors(instance)
            .map(|error| {
                (
                    error.schema_path().as_str().to_string(),
                    error
                        .absolute_keyword_location()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                    error.instance_path().as_str().to_string(),
                    error.to_string(),
                )
            })
            .collect()
    }

    fn first_error_report(
        schema: &Value,
        draft: Draft,
        instance: &Value,
    ) -> Option<(String, String, String, String, String)> {
        let validator = crate::options()
            .with_draft(draft)
            .build(schema)
            .expect("schema compiles");
        validator.validate(instance).err().map(|error| {
            (
                error.schema_path().as_str().to_string(),
                error.evaluation_path().as_str().to_string(),
                error
                    .absolute_keyword_location()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                error.instance_path().as_str().to_string(),
                error.to_string(),
            )
        })
    }

    fn output_report(schema: &Value, draft: Draft, instance: &Value) -> (Value, Value) {
        let validator = crate::options()
            .with_draft(draft)
            .build(schema)
            .expect("schema compiles");
        let evaluation = validator.evaluate(instance);
        (
            serde_json::to_value(evaluation.list()).expect("list output"),
            serde_json::to_value(evaluation.hierarchical()).expect("hierarchical output"),
        )
    }

    // `"type": [T]` is equivalent to `"type": T` but is not a single-type item schema, so it
    // compiles through the generic `items` path.
    #[test_case(Draft::Draft4, "number", &json!([1, "x", 2, null]))]
    #[test_case(Draft::Draft4, "string", &json!(["a", 1, "b", 2]))]
    #[test_case(Draft::Draft4, "boolean", &json!([true, 1, false, "x"]))]
    #[test_case(Draft::Draft4, "integer", &json!([1, 1.0, 2, 1.5]))]
    #[test_case(Draft::Draft6, "integer", &json!([1, 1.0, 2, 1.5]))]
    #[test_case(Draft::Draft7, "number", &json!([1, "x", 2, null]))]
    #[test_case(Draft::Draft7, "string", &json!(["a", 1, "b", 2]))]
    #[test_case(Draft::Draft7, "boolean", &json!([true, 1, false, "x"]))]
    #[test_case(Draft::Draft7, "integer", &json!([1, 1.0, 2, 1.5]))]
    #[test_case(Draft::Draft201909, "string", &json!(["a", 1, "b", 2]))]
    #[test_case(Draft::Draft201909, "integer", &json!([1, 1.0, 2, 1.5]))]
    #[test_case(Draft::Draft202012, "number", &json!([1, "x", 2, null]))]
    #[test_case(Draft::Draft202012, "string", &json!(["a", 1, "b", 2]))]
    #[test_case(Draft::Draft202012, "boolean", &json!([true, 1, false, "x"]))]
    #[test_case(Draft::Draft202012, "integer", &json!([1, 1.0, 2, 1.5]))]
    fn typed_items_report_like_generic_items(draft: Draft, ty: &str, instance: &Value) {
        let valid = Value::Array(
            instance
                .as_array()
                .expect("array instance")
                .iter()
                .step_by(2)
                .cloned()
                .collect(),
        );
        for (fast, generic) in [
            (
                json!({"items": {"type": ty}}),
                json!({"items": {"type": [ty]}}),
            ),
            (
                json!({"type": "array", "items": {"type": ty}}),
                json!({"type": "array", "items": {"type": [ty]}}),
            ),
            (
                json!({"type": "array", "minItems": 1, "maxItems": 9, "items": {"type": ty}}),
                json!({"type": "array", "minItems": 1, "maxItems": 9, "items": {"type": [ty]}}),
            ),
            // Reached through `$ref`, so evaluation paths differ from schema paths.
            (
                json!({"allOf": [{"$ref": "#/definitions/a"}], "definitions": {"a": {"items": {"type": ty}}}}),
                json!({"allOf": [{"$ref": "#/definitions/a"}], "definitions": {"a": {"items": {"type": [ty]}}}}),
            ),
            (
                json!({"allOf": [{"$ref": "#/definitions/a"}], "definitions": {"a": {"type": "array", "items": {"type": ty}}}}),
                json!({"allOf": [{"$ref": "#/definitions/a"}], "definitions": {"a": {"type": "array", "items": {"type": [ty]}}}}),
            ),
        ]
        .into_iter()
        .flat_map(|(fast, generic)| {
            [
                (with_id(draft, fast.clone()), with_id(draft, generic.clone())),
                (fast, generic),
            ]
        }) {
            for instance in [instance, &valid, &json!([]), &json!({})] {
                assert_eq!(
                    error_report(&fast, draft, instance),
                    error_report(&generic, draft, instance),
                    "{fast} errors for {instance}"
                );
                assert_eq!(
                    first_error_report(&fast, draft, instance),
                    first_error_report(&generic, draft, instance),
                    "{fast} first error for {instance}"
                );
                assert_eq!(
                    output_report(&fast, draft, instance),
                    output_report(&generic, draft, instance),
                    "{fast} output for {instance}"
                );
            }
        }
    }

    #[test]
    fn typed_items_error_locations() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root",
            "items": {"type": "string"}
        });
        assert_eq!(
            error_report(&schema, Draft::Draft202012, &json!([1])),
            [(
                "/items/type".to_string(),
                "https://example.com/root#/items/type".to_string(),
                "/0".to_string(),
                r#"1 is not of type "string""#.to_string(),
            )]
        );
        let (list, _) = output_report(&schema, Draft::Draft202012, &json!([1]));
        assert_eq!(
            list,
            json!({
                "valid": false,
                "details": [
                    {"valid": false, "evaluationPath": "", "schemaLocation": "https://example.com/root#", "instanceLocation": ""},
                    {"valid": false, "evaluationPath": "/items", "schemaLocation": "https://example.com/root#/items", "instanceLocation": "", "droppedAnnotations": true},
                    {"valid": false, "evaluationPath": "/items", "schemaLocation": "https://example.com/root#/items", "instanceLocation": "/0"},
                    {"valid": false, "evaluationPath": "/items/type", "schemaLocation": "https://example.com/root#/items/type", "instanceLocation": "/0", "errors": {"type": r#"1 is not of type "string""#}}
                ]
            })
        );
    }

    /// Drops the output units of `uniqueItems`, which the unfused schema carries only to block
    /// the fusion.
    fn without_unique_items(output: &mut Value) {
        match output {
            Value::Object(map) => {
                if let Some(Value::Array(details)) = map.get_mut("details") {
                    details.retain(|unit| {
                        !unit["evaluationPath"]
                            .as_str()
                            .is_some_and(|path| path.ends_with("/uniqueItems"))
                    });
                }
                map.values_mut().for_each(without_unique_items);
            }
            Value::Array(items) => items.iter_mut().for_each(without_unique_items),
            _ => {}
        }
    }

    #[test_case(Draft::Draft4)]
    #[test_case(Draft::Draft7)]
    #[test_case(Draft::Draft201909)]
    #[test_case(Draft::Draft202012)]
    fn array_shape_reports_like_unfused(draft: Draft) {
        let instances = [
            json!([]),
            json!([1]),
            json!([1, 2]),
            json!([1, 2, 3]),
            json!(["x"]),
            json!([-1, "x", 2]),
            json!("s"),
            json!({}),
        ];
        // Draft 4 has no boolean schemas.
        let never = if draft == Draft::Draft4 {
            json!({"not": {}})
        } else {
            json!(false)
        };
        for items in [
            json!({"type": "integer"}),
            json!({"type": "integer", "minimum": 0}),
            never,
        ] {
            for bounds in [
                json!({}),
                json!({"minItems": 2}),
                json!({"maxItems": 2}),
                json!({"minItems": 1, "maxItems": 2}),
            ] {
                let mut fused = json!({"type": "array", "items": items});
                fused
                    .as_object_mut()
                    .expect("object schema")
                    .extend(bounds.as_object().expect("object bounds").clone());
                let fused = with_id(draft, fused);
                let mut unfused = fused.clone();
                unfused["uniqueItems"] = json!(false);
                for instance in &instances {
                    assert_eq!(
                        error_report(&fused, draft, instance),
                        error_report(&unfused, draft, instance),
                        "{fused} errors for {instance}"
                    );
                    let (mut list, mut hierarchical) = output_report(&unfused, draft, instance);
                    without_unique_items(&mut list);
                    without_unique_items(&mut hierarchical);
                    assert_eq!(
                        output_report(&fused, draft, instance),
                        (list, hierarchical),
                        "{fused} output for {instance}"
                    );
                }
            }
        }
    }

    // Siblings sorting before and after the bounds, reached through `$ref` and a nested resource.
    #[test_case(Draft::Draft7, "definitions")]
    #[test_case(Draft::Draft202012, "$defs")]
    fn array_shape_reports_like_unfused_beside_siblings_and_refs(draft: Draft, defs: &str) {
        let fused = json!({
            "$id": "https://example.com/root",
            defs: {
                "shape": {
                    "type": "array",
                    "minItems": 2,
                    "maxItems": 3,
                    "items": {"type": "integer"},
                    "const": [1, 2],
                    "minProperties": 0,
                    "required": ["x"]
                },
                "nested": {
                    "$id": "nested",
                    "properties": {"p": {"$ref": format!("https://example.com/root#/{defs}/shape")}}
                }
            },
            "properties": {
                "x": {"$ref": format!("#/{defs}/shape")},
                "y": {"$ref": "nested"},
                "z": {"anyOf": [{"$ref": format!("#/{defs}/shape")}, {"type": "string"}]}
            }
        });
        let mut unfused = fused.clone();
        unfused[defs]["shape"]["uniqueItems"] = json!(false);
        for instance in [
            json!({"x": [1], "y": {"p": "s"}, "z": [1, 2, 3, 4, "q"]}),
            json!({"x": [1, 2], "y": {"p": [1, 2]}, "z": "ok"}),
            json!({"x": {}, "z": []}),
        ] {
            let (mut list, mut hierarchical) = output_report(&unfused, draft, &instance);
            without_unique_items(&mut list);
            without_unique_items(&mut hierarchical);
            assert_eq!(
                output_report(&fused, draft, &instance),
                (list, hierarchical),
                "output for {instance}"
            );
        }
    }

    // Fused `type:array` + optional min/maxItems + schema `items` (ArrayShapeValidator)
    #[test_case(&json!({"type": "array", "items": {"type": "number"}}), &json!([1, 2, 3]), true; "all valid")]
    #[test_case(&json!({"type": "array", "items": {"type": "number"}}), &json!([1, "x"]), false; "bad element")]
    #[test_case(&json!({"type": "array", "items": {"type": "number"}}), &json!("nope"), false; "non-array")]
    #[test_case(&json!({"type": "array", "items": {"type": "number"}}), &json!([]), true; "empty array")]
    #[test_case(&json!({"type": "array", "minItems": 2, "items": {"type": "number"}}), &json!([1]), false; "too short")]
    #[test_case(&json!({"type": "array", "minItems": 2, "items": {"type": "number"}}), &json!([1, 2]), true; "min satisfied")]
    #[test_case(&json!({"type": "array", "maxItems": 2, "items": {"type": "number"}}), &json!([1, 2, 3]), false; "too long")]
    #[test_case(&json!({"type": "array", "minItems": 1, "maxItems": 3, "items": {"type": "number"}}), &json!([1, 2]), true; "within bounds")]
    #[test_case(&json!({"type": "array", "items": {"type": "array", "items": {"type": "number"}}}), &json!([[1], [2, 3]]), true; "nested valid")]
    #[test_case(&json!({"type": "array", "items": {"type": "array", "items": {"type": "number"}}}), &json!([[1], ["x"]]), false; "nested bad element")]
    #[test_case(&json!({"type": "array", "items": {"type": "string"}}), &json!(["a", "b"]), true; "string valid")]
    #[test_case(&json!({"type": "array", "items": {"type": "string"}}), &json!(["a", 1]), false; "string bad element")]
    #[test_case(&json!({"type": "array", "items": {"type": "boolean"}}), &json!([true, false]), true; "boolean valid")]
    #[test_case(&json!({"type": "array", "items": {"type": "boolean"}}), &json!([true, 1]), false; "boolean bad element")]
    #[test_case(&json!({"type": "array", "items": {"type": "integer"}}), &json!([1, 2]), true; "integer valid")]
    #[test_case(&json!({"type": "array", "items": {"type": "integer"}}), &json!([1, 1.5]), false; "integer bad element")]
    #[test_case(&json!({"type": "array", "minItems": 2.0, "items": {"type": "number"}}), &json!([1]), false; "float minItems too short")]
    #[test_case(&json!({"type": "array", "minItems": 2.0, "items": {"type": "number"}}), &json!([1, 2]), true; "float minItems satisfied")]
    #[test_case(&json!({"type": "array", "items": {"type": "null"}}), &json!([null, null]), true; "null items valid")]
    #[test_case(&json!({"type": "array", "items": {"type": "null"}}), &json!([null, 1]), false; "null items bad element")]
    fn array_shape_is_valid(schema: &Value, instance: &Value, expected: bool) {
        let validator = crate::validator_for(schema).unwrap();
        assert_eq!(validator.is_valid(instance), expected);
        // `is_valid`, `validate`, `iter_errors`, and `evaluate` must agree on the verdict.
        assert_eq!(validator.validate(instance).is_ok(), expected);
        assert_eq!(validator.iter_errors(instance).next().is_none(), expected);
        assert_eq!(validator.evaluate(instance).flag().valid, expected);
    }

    // Draft 4 integer element semantics (1.0 is not an integer) route through the fused validator.
    #[test_case(&json!([1, 2]), true; "d4 integers valid")]
    #[test_case(&json!([1, 1.0]), false; "d4 float not integer")]
    #[test_case(&json!([1, "x"]), false; "d4 non-integer element")]
    fn array_shape_integer_draft4(instance: &Value, expected: bool) {
        let schema = json!({"type": "array", "items": {"type": "integer"}});
        if expected {
            tests_util::is_valid_with_draft4(&schema, instance);
        } else {
            tests_util::is_not_valid_with_draft4(&schema, instance);
        }
    }

    // Absorbed keywords keep their own schema location in errors.
    #[test_case(&json!({"type": "array", "items": {"type": "number"}}), &json!("x"), "/type"; "type location")]
    #[test_case(&json!({"type": "array", "minItems": 2, "items": {"type": "number"}}), &json!([1]), "/minItems"; "min location")]
    #[test_case(&json!({"type": "array", "maxItems": 1, "items": {"type": "number"}}), &json!([1, 2]), "/maxItems"; "max location")]
    #[test_case(&json!({"type": "array", "items": {"type": "number"}}), &json!([1, "x"]), "/items/type"; "element location")]
    fn array_shape_schema_location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    // A non-integer length keyword blocks fusion so the standalone validator still reports it.
    #[test]
    fn array_shape_invalid_length_keeps_error() {
        let schema = json!({"type": "array", "minItems": 1.5, "items": {"type": "number"}});
        assert!(crate::validator_for(&schema).is_err());
    }

    #[test]
    fn array_shape_yields_to_custom_length_keyword() {
        struct Accept;

        impl<'i> crate::Keyword<'i> for Accept {
            fn validate(&self, _: &'i Value) -> Result<(), crate::ValidationError<'i>> {
                Ok(())
            }

            fn is_valid(&self, _: &'i Value) -> bool {
                true
            }
        }

        let schema = json!({"type": "array", "minItems": 3, "items": {"type": "number"}});
        let validator = crate::options()
            .with_keyword("minItems", |_, _, _| Ok(Box::new(Accept)))
            .build(&schema)
            .unwrap();

        // The custom keyword owns `minItems`; the built-in bound must not also reject.
        assert!(validator.is_valid(&json!([1])));
        assert_eq!(validator.iter_errors(&json!([1])).count(), 0);
        // `type` and `items` still apply.
        assert!(!validator.is_valid(&json!([1, "x"])));
    }

    #[test]
    fn simple_type_items_respects_disabled_validation_vocabulary() {
        let meta = json!({
            "$id": "json-schema:///meta/no-validation",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/applicator": true,
                "https://json-schema.org/draft/2020-12/vocab/validation": false
            }
        });
        let registry = crate::Registry::new()
            .add("json-schema:///meta/no-validation", &meta)
            .unwrap()
            .prepare()
            .unwrap();
        let schema = json!({
            "$schema": "json-schema:///meta/no-validation",
            "type": "array",
            "items": {"type": "integer"}
        });
        let validator = crate::options()
            .with_registry(&registry)
            .build(&schema)
            .unwrap();
        assert!(validator.is_valid(&json!([1, "x"])));
    }

    #[test]
    fn array_shape_respects_disabled_applicator_vocabulary() {
        let meta = json!({
            "$id": "json-schema:///meta/no-applicator",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/validation": true
            }
        });
        let registry = crate::Registry::new()
            .add("json-schema:///meta/no-applicator", &meta)
            .unwrap()
            .prepare()
            .unwrap();
        let schema = json!({
            "$schema": "json-schema:///meta/no-applicator",
            "type": "array",
            "minItems": 2,
            "items": {"type": "integer"}
        });
        let validator = crate::options()
            .with_registry(&registry)
            .build(&schema)
            .unwrap();
        // `items` is inert, while `type` and `minItems` still apply.
        let verdicts: Vec<_> = [json!("x"), json!([1]), json!(["a", "b"])]
            .iter()
            .map(|instance| {
                (
                    validator.is_valid(instance),
                    validator.evaluate(instance).flag().valid,
                )
            })
            .collect();
        assert_eq!(verdicts, [(false, false), (false, false), (true, true)]);
    }

    fn parse_json(s: &str) -> Value {
        serde_json::from_str(s).unwrap()
    }

    // Specialized string type validator tests
    #[test_case(r#"{"items": {"type": "string"}}"#, r#"["a", "b", "c"]"#, true; "all strings valid")]
    #[test_case(r#"{"items": {"type": "string"}}"#, r#"["a", 1, "c"]"#, false; "mixed with number invalid")]
    #[test_case(r#"{"items": {"type": "string"}}"#, r"[]", true; "empty array valid")]
    #[test_case(r#"{"items": {"type": "string"}}"#, r#"[""]"#, true; "empty string valid")]
    #[test_case(r#"{"items": {"type": "string"}}"#, r"[null]", false; "null invalid")]
    #[test_case(r#"{"items": {"type": "string"}}"#, r"[true]", false; "boolean invalid")]
    fn items_string_type(schema_json: &str, instance_json: &str, expected: bool) {
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        if expected {
            tests_util::is_valid(&schema, &instance);
        } else {
            tests_util::is_not_valid(&schema, &instance);
        }
    }

    // Specialized number type validator tests
    #[test_case(r#"{"items": {"type": "number"}}"#, r"[1, 2.5, -3]", true; "all numbers valid")]
    #[test_case(r#"{"items": {"type": "number"}}"#, r#"[1, "2", 3]"#, false; "mixed with string invalid")]
    #[test_case(r#"{"items": {"type": "number"}}"#, r"[]", true; "empty array valid")]
    #[test_case(r#"{"items": {"type": "number"}}"#, r"[0]", true; "zero valid")]
    #[test_case(r#"{"items": {"type": "number"}}"#, r"[1.0]", true; "float valid")]
    #[test_case(r#"{"items": {"type": "number"}}"#, r"[null]", false; "null invalid")]
    #[test_case(r#"{"items": {"type": "number"}}"#, r"[9223372036854775807]", true; "i64 max valid")]
    #[test_case(r#"{"items": {"type": "number"}}"#, r"[-9223372036854775808]", true; "i64 min valid")]
    #[test_case(r#"{"items": {"type": "number"}}"#, r"[18446744073709551615]", true; "u64 max valid")]
    fn items_number_type(schema_json: &str, instance_json: &str, expected: bool) {
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        if expected {
            tests_util::is_valid(&schema, &instance);
        } else {
            tests_util::is_not_valid(&schema, &instance);
        }
    }

    // Specialized boolean type validator tests
    #[test_case(r#"{"items": {"type": "boolean"}}"#, r"[true, false]", true; "all booleans valid")]
    #[test_case(r#"{"items": {"type": "boolean"}}"#, r"[true, 1]", false; "mixed with number invalid")]
    #[test_case(r#"{"items": {"type": "boolean"}}"#, r"[]", true; "empty array valid")]
    #[test_case(r#"{"items": {"type": "boolean"}}"#, r"[null]", false; "null invalid")]
    #[test_case(r#"{"items": {"type": "boolean"}}"#, r#"["true"]"#, false; "string true invalid")]
    fn items_boolean_type(schema_json: &str, instance_json: &str, expected: bool) {
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        if expected {
            tests_util::is_valid(&schema, &instance);
        } else {
            tests_util::is_not_valid(&schema, &instance);
        }
    }

    // Specialized integer type validator tests (Draft 7+ semantics: 1.0 is integer)
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1, 2, 3]", true; "d7 all integers valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1, 2.5, 3]", false; "d7 float invalid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[]", true; "d7 empty array valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[0]", true; "d7 zero valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[-42]", true; "d7 negative valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1.0]", true; "d7 1.0 is integer")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[42.0]", true; "d7 42.0 is integer")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[-42.0]", true; "d7 neg 42.0 is integer")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[null]", false; "d7 null invalid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r#"["1"]"#, false; "d7 string invalid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[9223372036854775807]", true; "d7 i64 max valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[-9223372036854775808]", true; "d7 i64 min valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[18446744073709551615]", true; "d7 u64 max valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1e10]", true; "d7 scientific notation integer")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1e-10]", false; "d7 scientific small not integer")]
    fn items_integer_type_draft7(schema_json: &str, instance_json: &str, expected: bool) {
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        if expected {
            tests_util::is_valid(&schema, &instance);
        } else {
            tests_util::is_not_valid(&schema, &instance);
        }
    }

    // Draft 4 integer semantics: 1.0 is NOT an integer
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1, 2, 3]", true; "d4 all integers valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1, 2.5, 3]", false; "d4 float invalid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[]", true; "d4 empty array valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1.0]", false; "d4 1.0 is NOT integer")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[42.0]", false; "d4 42.0 is NOT integer")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[-42.0]", false; "d4 neg 42.0 is NOT integer")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[9223372036854775807]", true; "d4 i64 max valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[-9223372036854775808]", true; "d4 i64 min valid")]
    #[test_case(r#"{"items": {"type": "integer"}}"#, r"[18446744073709551615]", true; "d4 u64 max valid")]
    fn items_integer_type_draft4(schema_json: &str, instance_json: &str, expected: bool) {
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        if expected {
            tests_util::is_valid_with_draft4(&schema, &instance);
        } else {
            tests_util::is_not_valid_with_draft4(&schema, &instance);
        }
    }

    #[cfg(feature = "arbitrary-precision")]
    mod arbitrary_precision {
        use crate::tests_util;
        use serde_json::Value;
        use test_case::test_case;

        fn parse_json(s: &str) -> Value {
            serde_json::from_str(s).unwrap()
        }

        // Draft 7+ with huge integers
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[18446744073709551616]", true; "u64 max plus 1")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[18446744073709551616.0]", true; "u64 max plus 1 with .0")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[99999999999999999999]", true; "huge plain integer")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[99999999999999999999.0]", true; "huge integer with .0")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[-18446744073709551616]", true; "negative huge")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[-18446744073709551616.0]", true; "negative huge with .0")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[18446744073709551616.5]", false; "huge decimal")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[99999999999999999999.5]", false; "huge float")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1e1000]", true; "huge scientific notation")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1e1000001]", false; "infinity positive")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[-1e1000001]", false; "infinity negative")]
        fn items_integer_huge_draft7(schema_json: &str, instance_json: &str, expected: bool) {
            let schema = parse_json(schema_json);
            let instance = parse_json(instance_json);
            if expected {
                tests_util::is_valid(&schema, &instance);
            } else {
                tests_util::is_not_valid(&schema, &instance);
            }
        }

        // Draft 4 with huge integers (stricter: .0 is NOT integer)
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[18446744073709551616]", true; "u64 max plus 1")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[18446744073709551616.0]", false; "u64 max plus 1 with .0 NOT integer")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[99999999999999999999]", true; "huge plain integer")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[99999999999999999999.0]", false; "huge integer with .0 NOT integer")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[-18446744073709551616]", true; "negative huge")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[-18446744073709551616.0]", false; "negative huge with .0 NOT integer")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[18446744073709551616.5]", false; "huge decimal")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1e1000]", false; "huge scientific notation is not an integer in draft4")]
        #[test_case(r#"{"items": {"type": "integer"}}"#, r"[1e1000001]", false; "scientific notation past f64 is not an integer in draft4")]
        fn items_integer_huge_draft4(schema_json: &str, instance_json: &str, expected: bool) {
            let schema = parse_json(schema_json);
            let instance = parse_json(instance_json);
            if expected {
                tests_util::is_valid_with_draft4(&schema, &instance);
            } else {
                tests_util::is_not_valid_with_draft4(&schema, &instance);
            }
        }

        // Huge numbers for number type (all should be valid)
        #[test_case(r#"{"items": {"type": "number"}}"#, r"[18446744073709551616]", true; "huge int valid as number")]
        #[test_case(r#"{"items": {"type": "number"}}"#, r"[18446744073709551616.0]", true; "huge .0 valid as number")]
        #[test_case(r#"{"items": {"type": "number"}}"#, r"[18446744073709551616.5]", true; "huge float valid as number")]
        #[test_case(r#"{"items": {"type": "number"}}"#, r"[1e10000]", true; "infinity valid as number")]
        fn items_number_huge(schema_json: &str, instance_json: &str, expected: bool) {
            let schema = parse_json(schema_json);
            let instance = parse_json(instance_json);
            if expected {
                tests_util::is_valid(&schema, &instance);
            } else {
                tests_util::is_not_valid(&schema, &instance);
            }
        }
    }

    #[test]
    fn array_shape_absolute_keyword_locations() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/s.json",
            "type": "array",
            "minItems": 5,
            "maxItems": 1,
            "items": {"type": "string"}
        });
        tests_util::assert_absolute_keyword_locations(
            &schema,
            &json!([1]),
            &[
                ("minItems", "https://example.com/s.json#/minItems"),
                ("type", "https://example.com/s.json#/items/type"),
            ],
        );
        tests_util::assert_absolute_keyword_locations(
            &schema,
            &json!(["a", "b"]),
            &[
                ("minItems", "https://example.com/s.json#/minItems"),
                ("maxItems", "https://example.com/s.json#/maxItems"),
            ],
        );
        tests_util::assert_absolute_keyword_locations(
            &schema,
            &json!(1),
            &[("type", "https://example.com/s.json#/type")],
        );
    }

    #[test_case(
        &json!([1]),
        &[
            ("[1] has less than 2 items", "", "/minItems"),
            ("1 is not of type \"string\"", "/0", "/items/type"),
        ];
        "minItems with items"
    )]
    #[test_case(
        &json!(["a", "b", "c", "d"]),
        &[("[\"a\",\"b\",\"c\",\"d\"] has more than 3 items", "", "/maxItems")];
        "maxItems"
    )]
    #[test_case(&json!(1), &[("1 is not of type \"array\"", "", "/type")]; "type")]
    fn array_shape_error_locations(instance: &Value, expected: &[(&str, &str, &str)]) {
        tests_util::assert_error_locations(
            &json!({"type": "array", "minItems": 2, "maxItems": 3, "items": {"type": "string"}}),
            instance,
            expected,
        );
    }
}
