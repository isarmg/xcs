#[cfg(test)]
mod tests {
    use crate::{validator_for, Registry, JsonValueBackend, ValidationError};

    use super::Draft;
    use json_codec::{json, Value};
    use test_case::test_case;

    // A build error judges the schema itself, so its instance path is the location that failed
    // to compile.
    #[test_case(&json!({"type": "object", "properties": {"sku": {"type": "string", "pattern": "([unclosed"}}}), "/properties/sku/pattern")]
    #[test_case(&json!({"patternProperties": {"([unclosed": {"type": "string"}}}), "/patternProperties/([unclosed")]
    fn build_error_instance_path(schema: &Value, expected: &str) {
        let error = validator_for(schema).expect_err("Should fail to compile");
        assert_eq!(error.instance_path().as_str(), expected);
    }

    // A subschema's `$schema` switches the draft for that subschema, whatever the root draft.
    #[test_case(Draft::Draft4, &json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "prefixItems": [{"type": "string"}]}), false ; "2020-12 under draft 4")]
    #[test_case(Draft::Draft4, &json!({"$schema": "https://json-schema.org/draft/2019-09/schema", "items": [{"type": "string"}]}), false ; "2019-09 under draft 4")]
    #[test_case(Draft::Draft7, &json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "prefixItems": [{"type": "string"}]}), false ; "2020-12 under draft 7")]
    #[test_case(Draft::Draft4, &json!({"$schema": "http://json-schema.org/draft-07/schema#", "items": [{"type": "string"}]}), false ; "draft 7 under draft 4")]
    #[test_case(Draft::Draft201909, &json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "prefixItems": [{"type": "string"}]}), false ; "2020-12 under 2019-09")]
    #[test_case(Draft::Draft202012, &json!({"$schema": "http://json-schema.org/draft-04/schema#", "prefixItems": [{"type": "string"}]}), true ; "draft 4 under 2020-12")]
    fn nested_schema_switches_draft(root: Draft, pair: &Value, expected: bool) {
        let validator = crate::options()
            .with_draft(root)
            .build(&json!({"properties": {"pair": pair}}))
            .expect("Should build validator");
        assert_eq!(validator.is_valid(&json!({"pair": [1]})), expected);
    }

    // The fragment of an absolute keyword location is a JSON Pointer from the root of the
    // resource that holds the keyword, not from the root of the document embedding it.
    #[test_case(
        &json!({"properties": {"tree": {"$id": "https://example.com/tree", "type": "object", "properties": {"value": {"type": "string"}}}}}),
        &json!({"tree": {"value": 1}}),
        &["https://example.com/tree#/properties/value/type"];
        "draft 2020-12"
    )]
    #[test_case(
        &json!({"properties": {"tree": {"$id": "https://example.com/tree", "type": "object"}}}),
        &json!({"tree": 1}),
        &["https://example.com/tree#/type"];
        "keyword at resource root"
    )]
    #[test_case(
        &json!({"$schema": "https://json-schema.org/draft/2019-09/schema", "properties": {"tree": {"$id": "https://example.com/tree", "properties": {"value": {"type": "string"}}}}}),
        &json!({"tree": {"value": 1}}),
        &["https://example.com/tree#/properties/value/type"];
        "draft 2019-09"
    )]
    #[test_case(
        &json!({"$schema": "http://json-schema.org/draft-07/schema#", "properties": {"tree": {"$id": "https://example.com/tree", "properties": {"value": {"type": "string"}}}}}),
        &json!({"tree": {"value": 1}}),
        &["https://example.com/tree#/properties/value/type"];
        "draft 7"
    )]
    #[test_case(
        &json!({"$schema": "http://json-schema.org/draft-06/schema#", "properties": {"tree": {"$id": "https://example.com/tree", "properties": {"value": {"type": "string"}}}}}),
        &json!({"tree": {"value": 1}}),
        &["https://example.com/tree#/properties/value/type"];
        "draft 6"
    )]
    #[test_case(
        &json!({"$schema": "http://json-schema.org/draft-04/schema#", "properties": {"tree": {"id": "https://example.com/tree", "properties": {"value": {"type": "string"}}}}}),
        &json!({"tree": {"value": 1}}),
        &["https://example.com/tree#/properties/value/type"];
        "draft 4"
    )]
    #[test_case(
        &json!({"$id": "https://example.com/root.json", "properties": {"tree": {"$id": "tree.json", "properties": {"value": {"type": "string"}}}}}),
        &json!({"tree": {"value": 1}}),
        &["https://example.com/tree.json#/properties/value/type"];
        "relative id"
    )]
    #[test_case(
        &json!({
            "$id": "https://example.com/root.json",
            "properties": {
                "outer": {
                    "$id": "outer/",
                    "properties": {
                        "inner": {"$id": "inner.json", "properties": {"value": {"type": "string"}}},
                        "count": {"type": "integer"}
                    }
                },
                "name": {"type": "string"}
            }
        }),
        &json!({"outer": {"inner": {"value": 1}, "count": "x"}, "name": 1}),
        &[
            "https://example.com/outer/#/properties/count/type",
            "https://example.com/outer/inner.json#/properties/value/type",
            "https://example.com/root.json#/properties/name/type",
        ];
        "id nested in id"
    )]
    #[test_case(
        &json!({"$schema": "http://json-schema.org/draft-07/schema#", "$id": "https://example.com/root", "properties": {"tree": {"$id": "#tree", "properties": {"value": {"type": "string"}}}}}),
        &json!({"tree": {"value": 1}}),
        &["https://example.com/root#/properties/tree/properties/value/type"];
        "draft 7 fragment-only id names no resource"
    )]
    #[test_case(
        &json!({"$schema": "http://json-schema.org/draft-07/schema#", "$id": "https://example.com/root", "properties": {"tree": {"$id": "https://example.com/tree", "$ref": "#/definitions/leaf"}}, "definitions": {"leaf": {"type": "string"}}}),
        &json!({"tree": 1}),
        &["https://example.com/root#/definitions/leaf/type"];
        "draft 7 id beside ref names no resource"
    )]
    fn absolute_keyword_location_in_nested_resource(
        schema: &Value,
        instance: &Value,
        expected: &[&str],
    ) {
        let validator = validator_for(schema).expect("Should build validator");
        let mut locations: Vec<String> = validator
            .iter_errors(instance)
            .map(|error| {
                error
                    .absolute_keyword_location()
                    .expect("Absolute keyword location")
                    .to_string()
            })
            .collect();
        locations.sort();
        assert_eq!(locations, expected);
        let evaluation = validator.evaluate(instance);
        let mut locations: Vec<String> = evaluation
            .iter_errors()
            .map(|entry| {
                entry
                    .absolute_keyword_location
                    .expect("Absolute keyword location")
                    .to_string()
            })
            .collect();
        locations.sort();
        assert_eq!(locations, expected);
    }

    #[test_case(
        &json!({"properties": {"tree": {"$id": "https://example.com/tree", "properties": {"value": false}}}}),
        &json!({"tree": {"value": 1}}),
        &[("https://example.com/tree#/properties/value", Some("https://example.com/tree#/properties/value"))],
        &[Some("https://example.com/tree#/properties/value")];
        "false subschema in nested resource"
    )]
    #[test_case(
        &json!({"properties": {"tree": {"$id": "https://example.com/tree", "unevaluatedProperties": false}}}),
        &json!({"tree": {"a": 1, "b": 2}}),
        &[
            ("https://example.com/tree#/unevaluatedProperties", Some("https://example.com/tree#/unevaluatedProperties")),
            ("https://example.com/tree#/unevaluatedProperties", Some("https://example.com/tree#/unevaluatedProperties")),
            ("https://example.com/tree#/unevaluatedProperties", Some("https://example.com/tree#/unevaluatedProperties")),
        ],
        &[Some("https://example.com/tree#/unevaluatedProperties")];
        "unevaluatedProperties false in nested resource"
    )]
    #[test_case(
        &json!({"$id": "https://example.com/root", "$ref": "#/$defs/never", "$defs": {"never": false}}),
        &json!(1),
        &[("https://example.com/root#/$defs/never", Some("https://example.com/root#/$defs/never"))],
        &[Some("https://example.com/root#/$defs/never")];
        "ref to false in same resource"
    )]
    #[test_case(
        &json!({"$id": "https://example.com/root", "$ref": "other#/$defs/never", "$defs": {"other": {"$id": "other", "$defs": {"never": false}}}}),
        &json!(1),
        &[("https://example.com/other#/$defs/never", Some("https://example.com/other#/$defs/never"))],
        &[Some("https://example.com/other#/$defs/never")];
        "ref to false in other resource"
    )]
    #[test_case(
        &json!({"properties": {"value": false}}),
        &json!({"value": 1}),
        &[("/properties/value", None)],
        &[None];
        "false subschema without id"
    )]
    fn false_schema_error_locations(
        schema: &Value,
        instance: &Value,
        evaluate: &[(&str, Option<&str>)],
        iter_errors: &[Option<&str>],
    ) {
        let validator = validator_for(schema).expect("Should build validator");
        let evaluation = validator.evaluate(instance);
        let locations: Vec<(&str, Option<String>)> = evaluation
            .iter_errors()
            .map(|entry| {
                (
                    entry.schema_location,
                    entry.absolute_keyword_location.map(ToString::to_string),
                )
            })
            .collect();
        let expected: Vec<(&str, Option<String>)> = evaluate
            .iter()
            .map(|(schema_location, absolute)| (*schema_location, absolute.map(str::to_string)))
            .collect();
        assert_eq!(locations, expected);
        let locations: Vec<Option<String>> = validator
            .iter_errors(instance)
            .map(|error| error.absolute_keyword_location().map(ToString::to_string))
            .collect();
        let expected: Vec<Option<String>> = iter_errors
            .iter()
            .map(|absolute| absolute.map(str::to_string))
            .collect();
        assert_eq!(locations, expected);
    }

    #[test_case(
        &json!({"$id": "https://example.com/tree", "properties": {"a": {"type": "string"}, "b": {"type": "string"}}, "required": ["a", "b"]}),
        &json!({}),
        &["", "/properties", "https://example.com/tree#", "https://example.com/tree#/properties", "https://example.com/tree#/required"];
        "required"
    )]
    #[test_case(
        &json!({"$id": "https://example.com/tree", "minLength": 1, "maxLength": 2}),
        &json!("abc"),
        &["", "/properties", "https://example.com/tree#", "https://example.com/tree#/minLength", "https://example.com/tree#/maxLength"];
        "maxLength beside minLength"
    )]
    #[test_case(
        &json!({"$id": "https://example.com/tree", "type": "array", "minItems": 2, "maxItems": 0, "items": {"type": "string"}}),
        &json!([1]),
        &["", "/properties", "https://example.com/tree#", "https://example.com/tree#/type", "https://example.com/tree#/minItems", "https://example.com/tree#/maxItems", "https://example.com/tree#/items", "https://example.com/tree#/items", "https://example.com/tree#/items/type"];
        "array bounds beside items"
    )]
    #[test_case(
        &json!({"$id": "https://example.com/tree", "type": "array", "items": {"type": "string"}}),
        &json!(1),
        &["", "/properties", "https://example.com/tree#", "https://example.com/tree#/type", "https://example.com/tree#/items"];
        "type beside items"
    )]
    #[test_case(
        &json!({"$id": "https://example.com/tree", "patternProperties": {"^a": {}}, "additionalProperties": false}),
        &json!({"a": 1, "b": 1}),
        &["", "/properties", "https://example.com/tree#", "https://example.com/tree#/additionalProperties", "https://example.com/tree#/patternProperties/%5Ea", "https://example.com/tree#/patternProperties"];
        "patternProperties beside additionalProperties"
    )]
    fn list_schema_locations_in_nested_resource(tree: &Value, instance: &Value, expected: &[&str]) {
        let validator =
            validator_for(&json!({"properties": {"tree": tree}})).expect("Should build validator");
        let evaluation = validator.evaluate(&json!({"tree": instance}));
        let list = json_codec::to_value(evaluation.list()).expect("List output");
        let locations: Vec<&str> = list["details"]
            .as_array()
            .expect("List output details")
            .iter()
            .map(|entry| entry["schemaLocation"].as_str().expect("Schema location"))
            .collect();
        assert_eq!(locations, expected);
    }

    #[test]
    fn output_locations_in_nested_resource() {
        let validator = validator_for(&json!({
            "properties": {
                "tree": {"$id": "https://example.com/tree", "properties": {"value": {"type": "string"}}}
            }
        }))
        .expect("Should build validator");

        let evaluation = validator.evaluate(&json!({"tree": {"value": 1}}));
        let list = json_codec::to_value(evaluation.list()).expect("List output");
        let locations: Vec<&str> = list["details"]
            .as_array()
            .expect("List output details")
            .iter()
            .map(|entry| entry["schemaLocation"].as_str().expect("Schema location"))
            .collect();
        assert_eq!(
            locations,
            [
                "",
                "/properties",
                "https://example.com/tree#",
                "https://example.com/tree#/properties",
                "https://example.com/tree#/properties/value",
                "https://example.com/tree#/properties/value/type",
            ]
        );

        let evaluation = validator.evaluate(&json!({"tree": {"value": "leaf"}}));
        let annotations: Vec<Option<String>> = evaluation
            .iter_annotations()
            .map(|entry| entry.absolute_keyword_location.map(ToString::to_string))
            .collect();
        assert_eq!(
            annotations,
            [
                None,
                Some("https://example.com/tree#/properties".to_string())
            ]
        );
    }

    #[test_case(
        &json!({"$id": "https://example.com/root", "patternProperties": {"^a": {}}, "additionalProperties": false}),
        &json!({"a": 1}),
        &[("https://example.com/root#/patternProperties", Some("https://example.com/root#/patternProperties"))];
        "patternProperties beside additionalProperties at resource root"
    )]
    #[test_case(
        &json!({"properties": {"tree": {"$id": "https://example.com/tree", "patternProperties": {"^a": {}}, "additionalProperties": false}}}),
        &json!({"tree": {"a": 1}}),
        &[
            ("/properties", None),
            ("https://example.com/tree#/patternProperties", Some("https://example.com/tree#/patternProperties")),
        ];
        "patternProperties beside additionalProperties in nested resource"
    )]
    #[test_case(
        &json!({"properties": {"tree": {"$id": "https://example.com/tree", "unknown": 1}}}),
        &json!({"tree": 1}),
        &[
            ("/properties", None),
            ("https://example.com/tree#", Some("https://example.com/tree#")),
        ];
        "unknown keyword at nested resource root"
    )]
    fn annotation_locations(schema: &Value, instance: &Value, expected: &[(&str, Option<&str>)]) {
        let validator = validator_for(schema).expect("Should build validator");
        let evaluation = validator.evaluate(instance);
        let locations: Vec<(&str, Option<String>)> = evaluation
            .iter_annotations()
            .map(|entry| {
                (
                    entry.schema_location,
                    entry.absolute_keyword_location.map(ToString::to_string),
                )
            })
            .collect();
        let expected: Vec<(&str, Option<String>)> = expected
            .iter()
            .map(|(schema_location, absolute)| (*schema_location, absolute.map(str::to_string)))
            .collect();
        assert_eq!(locations, expected);
    }

    fn property_with_space() -> crate::Validator {
        validator_for(&json!({
            "$id": "https://example.com/root",
            "properties": {"s p": {"type": "string"}}
        }))
        .expect("Should build validator")
    }

    #[test]
    fn list_output_encodes_schema_locations() {
        let validator = property_with_space();
        let evaluation = validator.evaluate(&json!({"s p": 1}));
        assert_eq!(
            json_codec::to_value(evaluation.list()).expect("List output"),
            json!({
                "valid": false,
                "details": [
                    {
                        "valid": false,
                        "evaluationPath": "",
                        "schemaLocation": "https://example.com/root#",
                        "instanceLocation": ""
                    },
                    {
                        "valid": false,
                        "evaluationPath": "/properties",
                        "schemaLocation": "https://example.com/root#/properties",
                        "instanceLocation": "",
                        "droppedAnnotations": ["s p"]
                    },
                    {
                        "valid": false,
                        "evaluationPath": "/properties/s p",
                        "schemaLocation": "https://example.com/root#/properties/s%20p",
                        "instanceLocation": "/s p"
                    },
                    {
                        "valid": false,
                        "evaluationPath": "/properties/s p/type",
                        "schemaLocation": "https://example.com/root#/properties/s%20p/type",
                        "instanceLocation": "/s p",
                        "errors": {"type": "1 is not of type \"string\""}
                    }
                ]
            })
        );
    }

    #[test]
    fn hierarchical_output_encodes_schema_locations() {
        let validator = property_with_space();
        let evaluation = validator.evaluate(&json!({"s p": 1}));
        assert_eq!(
            json_codec::to_value(evaluation.hierarchical()).expect("Hierarchical output"),
            json!({
                "valid": false,
                "evaluationPath": "",
                "schemaLocation": "https://example.com/root#",
                "instanceLocation": "",
                "details": [
                    {
                        "valid": false,
                        "evaluationPath": "/properties",
                        "schemaLocation": "https://example.com/root#/properties",
                        "instanceLocation": "",
                        "droppedAnnotations": ["s p"],
                        "details": [
                            {
                                "valid": false,
                                "evaluationPath": "/properties/s p",
                                "schemaLocation": "https://example.com/root#/properties/s%20p",
                                "instanceLocation": "/s p",
                                "details": [
                                    {
                                        "valid": false,
                                        "evaluationPath": "/properties/s p/type",
                                        "schemaLocation": "https://example.com/root#/properties/s%20p/type",
                                        "instanceLocation": "/s p",
                                        "errors": {"type": "1 is not of type \"string\""}
                                    }
                                ]
                            }
                        ]
                    }
                ]
            })
        );
    }

    #[test]
    fn annotation_locations_are_encoded() {
        let validator = validator_for(&json!({
            "$id": "https://example.com/root",
            "properties": {"s p": {"properties": {"a b": true}}}
        }))
        .expect("Should build validator");
        let evaluation = validator.evaluate(&json!({"s p": {"a b": 1}}));
        let locations: Vec<(&str, Option<String>, &str)> = evaluation
            .iter_annotations()
            .map(|entry| {
                (
                    entry.schema_location,
                    entry.absolute_keyword_location.map(ToString::to_string),
                    entry.instance_location.as_str(),
                )
            })
            .collect();
        assert_eq!(
            locations,
            [
                (
                    "https://example.com/root#/properties",
                    Some("https://example.com/root#/properties".to_string()),
                    ""
                ),
                (
                    "https://example.com/root#/properties/s%20p/properties",
                    Some("https://example.com/root#/properties/s%20p/properties".to_string()),
                    "/s p"
                ),
            ]
        );
        assert_eq!(
            json_codec::to_value(evaluation.list()).expect("List output"),
            json!({
                "valid": true,
                "details": [
                    {
                        "valid": true,
                        "evaluationPath": "",
                        "schemaLocation": "https://example.com/root#",
                        "instanceLocation": ""
                    },
                    {
                        "valid": true,
                        "evaluationPath": "/properties",
                        "schemaLocation": "https://example.com/root#/properties",
                        "instanceLocation": "",
                        "annotations": ["s p"]
                    },
                    {
                        "valid": true,
                        "evaluationPath": "/properties/s p",
                        "schemaLocation": "https://example.com/root#/properties/s%20p",
                        "instanceLocation": "/s p"
                    },
                    {
                        "valid": true,
                        "evaluationPath": "/properties/s p/properties",
                        "schemaLocation": "https://example.com/root#/properties/s%20p/properties",
                        "instanceLocation": "/s p",
                        "annotations": ["a b"]
                    },
                    {
                        "valid": true,
                        "evaluationPath": "/properties/s p/properties/a b",
                        "schemaLocation": "https://example.com/root#/properties/s%20p/properties/a%20b",
                        "instanceLocation": "/s p/a b"
                    }
                ]
            })
        );
    }

    #[test_case("s p", "s p", "s%20p")]
    #[test_case("a%b", "a%b", "a%25b")]
    #[test_case("q\"t", "q\"t", "q%22t")]
    #[test_case("c^d", "c^d", "c%5Ed")]
    #[test_case("[x]", "[x]", "%5Bx%5D")]
    #[test_case("{y}", "{y}", "%7By%7D")]
    #[test_case("p|q", "p|q", "p%7Cq")]
    #[test_case("b\\s", "b\\s", "b%5Cs")]
    #[test_case("<>", "<>", "%3C%3E")]
    #[test_case("`b", "`b", "%60b")]
    #[test_case("#h", "#h", "%23h")]
    #[test_case("é", "é", "%C3%A9")]
    #[test_case("t~e", "t~0e", "t~0e")]
    #[test_case("s/l", "s~1l", "s~1l")]
    fn error_locations_agree_on_encoding(name: &str, pointer: &str, encoded: &str) {
        let validator = validator_for(&json!({
            "$id": "https://example.com/root",
            "properties": {name: {"type": "string"}}
        }))
        .expect("Should build validator");
        let instance = json!({name: 1});
        let expected = vec![(
            format!("/properties/{pointer}/type"),
            format!("https://example.com/root#/properties/{encoded}/type"),
            format!("/{pointer}"),
        )];

        let from_iter_errors: Vec<(String, String, String)> = validator
            .iter_errors(&instance)
            .map(|error| {
                (
                    error.schema_path().to_string(),
                    error
                        .absolute_keyword_location()
                        .expect("Absolute keyword location")
                        .to_string(),
                    error.instance_path().to_string(),
                )
            })
            .collect();
        assert_eq!(from_iter_errors, expected);

        let evaluation = validator.evaluate(&instance);
        let from_entries: Vec<(String, Option<String>, String)> = evaluation
            .iter_errors()
            .map(|entry| {
                (
                    entry.schema_location.to_string(),
                    entry.absolute_keyword_location.map(ToString::to_string),
                    entry.instance_location.to_string(),
                )
            })
            .collect();
        assert_eq!(
            from_entries,
            [(
                expected[0].1.clone(),
                Some(expected[0].1.clone()),
                expected[0].2.clone()
            )]
        );

        let list = json_codec::to_value(evaluation.list()).expect("List output");
        let from_list: Vec<(String, String, String)> = list["details"]
            .as_array()
            .expect("List output details")
            .iter()
            .filter(|entry| entry.get("errors").is_some())
            .map(|entry| {
                let field = |key: &str| entry[key].as_str().expect("Location").to_string();
                (
                    field("evaluationPath"),
                    field("schemaLocation"),
                    field("instanceLocation"),
                )
            })
            .collect();
        assert_eq!(from_list, expected);
    }

    #[test]
    fn ref_target_location_is_encoded() {
        let validator = validator_for(&json!({
            "$id": "https://example.com/root",
            "$ref": "#/$defs/s%20p",
            "$defs": {"s p": {"type": "string"}}
        }))
        .expect("Should build validator");
        let evaluation = validator.evaluate(&json!(1));
        let list = json_codec::to_value(evaluation.list()).expect("List output");
        let locations: Vec<(&str, &str)> = list["details"]
            .as_array()
            .expect("List output details")
            .iter()
            .map(|entry| {
                (
                    entry["evaluationPath"].as_str().expect("Evaluation path"),
                    entry["schemaLocation"].as_str().expect("Schema location"),
                )
            })
            .collect();
        assert_eq!(
            locations,
            [
                ("", "https://example.com/root#"),
                ("/$ref", "https://example.com/root#/$defs/s%20p"),
                ("/$ref/type", "https://example.com/root#/$defs/s%20p/type"),
            ]
        );
        let absolute: Vec<Option<String>> = validator
            .iter_errors(&json!(1))
            .map(|error| error.absolute_keyword_location().map(ToString::to_string))
            .collect();
        assert_eq!(
            absolute,
            [Some(
                "https://example.com/root#/$defs/s%20p/type".to_string()
            )]
        );
    }

    const DRAFT4: &str = "http://json-schema.org/draft-04/schema#";
    const DRAFT6: &str = "http://json-schema.org/draft-06/schema#";
    const DRAFT7: &str = "http://json-schema.org/draft-07/schema#";
    const DRAFT202012: &str = "https://json-schema.org/draft/2020-12/schema";

    fn with_embedded_resource(root: &str, resource: &Value) -> Value {
        json!({"$schema": root, "properties": {"a": resource}})
    }

    // An embedded resource is checked against the meta-schema of its own `$schema`, not the root's.
    #[test_case(DRAFT202012, &json!({"$id": "https://example.com/a", "$schema": DRAFT4, "items": [{"type": "string"}]}), &json!(["x"]), &json!([1]) ; "draft 4 tuple items under 2020-12")]
    #[test_case(DRAFT202012, &json!({"$id": "https://example.com/a", "$schema": DRAFT4, "minimum": 5, "exclusiveMinimum": true}), &json!(6), &json!(5) ; "draft 4 boolean exclusiveMinimum under 2020-12")]
    #[test_case(DRAFT6, &json!({"$id": "https://example.com/a", "$schema": DRAFT4, "minimum": 5, "exclusiveMinimum": true}), &json!(6), &json!(5) ; "draft 4 boolean exclusiveMinimum under draft 6")]
    #[test_case(DRAFT4, &json!({"$id": "https://example.com/a", "$schema": DRAFT202012, "exclusiveMinimum": 5}), &json!(6), &json!(5) ; "2020-12 numeric exclusiveMinimum under draft 4")]
    fn embedded_resource_follows_own_meta_schema(
        root: &str,
        resource: &Value,
        valid: &Value,
        invalid: &Value,
    ) {
        let validator =
            validator_for(&with_embedded_resource(root, resource)).expect("Should build validator");
        assert_eq!(
            (
                validator.is_valid(&json!({"a": valid})),
                validator.is_valid(&json!({"a": invalid}))
            ),
            (true, false)
        );
    }

    // The error points at the offending keyword inside the embedded resource.
    #[test_case(DRAFT7, &json!({"$id": "https://example.com/a", "$schema": DRAFT202012, "minContains": -1}), "/properties/a/minContains" ; "2020-12 under draft 7")]
    #[test_case(DRAFT202012, &json!({"$id": "https://example.com/a", "$schema": DRAFT4, "minimum": 1, "exclusiveMinimum": 5}), "/properties/a/exclusiveMinimum" ; "draft 4 under 2020-12")]
    #[test_case(DRAFT202012, &json!({"$id": "https://example.com/a", "$schema": DRAFT4, "properties": {"b": {"$id": "https://example.com/b", "$schema": DRAFT202012, "minContains": -1}}}), "/properties/a/properties/b/minContains" ; "2020-12 under draft 4 under 2020-12")]
    fn embedded_resource_rejected_by_own_meta_schema(root: &str, resource: &Value, expected: &str) {
        let error = validator_for(&with_embedded_resource(root, resource))
            .expect_err("Should reject the embedded resource");
        assert_eq!(error.instance_path().as_str(), expected);
    }

    // Without `$id` the subschema is not a resource root, so the root's meta-schema still applies.
    #[test]
    fn nested_schema_without_id_follows_root_meta_schema() {
        let error = validator_for(&with_embedded_resource(
            DRAFT202012,
            &json!({"$schema": DRAFT4, "items": [{"type": "string"}]}),
        ))
        .expect_err("Should reject array-form `items` under 2020-12");
        assert_eq!(error.instance_path().as_str(), "/properties/a/items");
    }

    #[test_case(crate::is_valid ; "autodetect")]
    #[test_case(crate::draft4::is_valid ; "draft4")]
    #[test_case(crate::draft6::is_valid ; "draft6")]
    #[test_case(crate::draft7::is_valid ; "draft7")]
    #[test_case(crate::draft201909::is_valid ; "draft201909")]
    #[test_case(crate::draft202012::is_valid ; "draft202012")]
    fn test_is_valid(is_valid_fn: fn(&json_codec::Value, &json_codec::Value) -> bool) {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            },
            "required": ["name"]
        });

        let valid_instance = json!({
            "name": "John Doe",
            "age": 30
        });

        let invalid_instance = json!({
            "age": -5
        });

        assert!(is_valid_fn(&schema, &valid_instance));
        assert!(!is_valid_fn(&schema, &invalid_instance));
    }

    #[test_case(crate::validate ; "autodetect")]
    #[test_case(crate::draft4::validate ; "draft4")]
    #[test_case(crate::draft6::validate ; "draft6")]
    #[test_case(crate::draft7::validate ; "draft7")]
    #[test_case(crate::draft201909::validate ; "draft201909")]
    #[test_case(crate::draft202012::validate ; "draft202012")]
    fn test_validate(
        validate_fn: for<'i> fn(
            &json_codec::Value,
            &'i json_codec::Value,
        ) -> Result<(), ValidationError<'i>>,
    ) {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            },
            "required": ["name"]
        });

        let valid_instance = json!({
            "name": "John Doe",
            "age": 30
        });

        let invalid_instance = json!({
            "age": -5
        });

        assert!(validate_fn(&schema, &valid_instance).is_ok());
        assert!(validate_fn(&schema, &invalid_instance).is_err());
    }

    #[test]
    fn test_evaluate() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            },
            "required": ["name"]
        });

        let valid_instance = json!({
            "name": "John Doe",
            "age": 30
        });

        let invalid_instance = json!({
            "age": -5
        });

        let valid_eval = crate::evaluate(&schema, &valid_instance);
        assert!(valid_eval.flag().valid);

        let invalid_eval = crate::evaluate(&schema, &invalid_instance);
        assert!(!invalid_eval.flag().valid);
        let errors: Vec<_> = invalid_eval.iter_errors().collect();
        assert!(!errors.is_empty());
    }

    #[test_case(crate::meta::validate, crate::meta::is_valid ; "autodetect")]
    #[test_case(crate::draft4::meta::validate, crate::draft4::meta::is_valid ; "draft4")]
    #[test_case(crate::draft6::meta::validate, crate::draft6::meta::is_valid ; "draft6")]
    #[test_case(crate::draft7::meta::validate, crate::draft7::meta::is_valid ; "draft7")]
    #[test_case(crate::draft201909::meta::validate, crate::draft201909::meta::is_valid ; "draft201909")]
    #[test_case(crate::draft202012::meta::validate, crate::draft202012::meta::is_valid ; "draft202012")]
    fn test_meta_validation(
        validate_fn: fn(&json_codec::Value) -> Result<(), ValidationError>,
        is_valid_fn: fn(&json_codec::Value) -> bool,
    ) {
        let valid = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            },
            "required": ["name"]
        });

        let invalid = json!({
            "type": "invalid_type",
            "minimum": "not_a_number",
            "required": true  // should be an array
        });

        assert!(validate_fn(&valid).is_ok());
        assert!(validate_fn(&invalid).is_err());
        assert!(is_valid_fn(&valid));
        assert!(!is_valid_fn(&invalid));
    }

    #[test_case(&json!({"type": "object", "required": ["name"]}), true ; "valid")]
    #[test_case(&json!({"type": "invalid_type", "required": true}), false ; "invalid")]
    #[test_case(&json!(true), true ; "boolean schema")]
    #[test_case(&json!({"$schema": "http://json-schema.org/draft-04/schema#", "minimum": 5, "exclusiveMinimum": true}), true ; "draft4 boolean exclusive minimum")]
    #[test_case(&json!({"$schema": "http://json-schema.org/draft-04/schema#", "exclusiveMinimum": 5}), false ; "draft4 numeric exclusive minimum")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "exclusiveMinimum": 5}), true ; "draft2020-12 numeric exclusive minimum")]
    fn test_meta_validate_for(schema: &json_codec::Value, expected: bool) {
        let result = crate::meta::validate_for::<JsonValueBackend>(schema);
        assert_eq!(result.is_ok(), expected);
        assert_eq!(result.is_ok(), crate::meta::validate(schema).is_ok());
        assert_eq!(
            crate::meta::is_valid_for::<JsonValueBackend>(schema).expect("Meta-schema is resolvable"),
            expected
        );
    }

    #[test]
    fn test_meta_validate_for_unresolvable_meta_schema() {
        let schema = json!({"$schema": "htt://json-schema.org/draft-07/schema"});
        let error = crate::meta::validate_for::<JsonValueBackend>(&schema)
            .expect_err("Unresolvable meta-schema should fail");
        assert!(matches!(
            error.kind(),
            crate::error::ValidationErrorKind::Referencing(_)
        ));
        let error = crate::meta::is_valid_for::<JsonValueBackend>(&schema)
            .expect_err("Unresolvable meta-schema should fail");
        assert!(matches!(
            error.kind(),
            crate::error::ValidationErrorKind::Referencing(_)
        ));
    }

    #[test]
    fn test_exclusive_minimum_across_drafts() {
        // In Draft 4, exclusiveMinimum is a boolean modifier for minimum
        let draft4_schema = json!({
            "$schema": "http://json-schema.org/draft-04/schema#",
            "minimum": 5,
            "exclusiveMinimum": true
        });
        assert!(crate::meta::is_valid(&draft4_schema));
        assert!(crate::meta::validate(&draft4_schema).is_ok());

        // This is invalid in Draft 4 (exclusiveMinimum must be boolean)
        let invalid_draft4 = json!({
            "$schema": "http://json-schema.org/draft-04/schema#",
            "exclusiveMinimum": 5
        });
        assert!(!crate::meta::is_valid(&invalid_draft4));
        assert!(crate::meta::validate(&invalid_draft4).is_err());

        // In Draft 6 and later, exclusiveMinimum is a numeric value
        let drafts = [
            "http://json-schema.org/draft-06/schema#",
            "http://json-schema.org/draft-07/schema#",
            "https://json-schema.org/draft/2019-09/schema",
            "https://json-schema.org/draft/2020-12/schema",
        ];

        for uri in drafts {
            // Valid in Draft 6+ (numeric exclusiveMinimum)
            let valid_schema = json!({
                "$schema": uri,
                "exclusiveMinimum": 5
            });
            assert!(
                crate::meta::is_valid(&valid_schema),
                "Schema should be valid for {uri}"
            );
            assert!(
                crate::meta::validate(&valid_schema).is_ok(),
                "Schema validation should succeed for {uri}",
            );

            // Invalid in Draft 6+ (can't use boolean with minimum)
            let invalid_schema = json!({
                "$schema": uri,
                "minimum": 5,
                "exclusiveMinimum": true
            });
            assert!(
                !crate::meta::is_valid(&invalid_schema),
                "Schema should be invalid for {uri}",
            );
            assert!(
                crate::meta::validate(&invalid_schema).is_err(),
                "Schema validation should fail for {uri}",
            );
        }
    }

    #[test_case(
        "http://json-schema.org/draft-04/schema#",
        true,
        5,
        true ; "draft4 valid"
    )]
    #[test_case(
        "http://json-schema.org/draft-04/schema#",
        5,
        true,
        false ; "draft4 invalid"
    )]
    #[test_case(
        "http://json-schema.org/draft-06/schema#",
        5,
        true,
        false ; "draft6 invalid"
    )]
    #[test_case(
        "http://json-schema.org/draft-07/schema#",
        5,
        true,
        false ; "draft7 invalid"
    )]
    #[test_case(
        "https://json-schema.org/draft/2019-09/schema",
        5,
        true,
        false ; "draft2019-09 invalid"
    )]
    #[test_case(
        "https://json-schema.org/draft/2020-12/schema",
        5,
        true,
        false ; "draft2020-12 invalid"
    )]
    fn test_exclusive_minimum_detection(
        schema_uri: &str,
        exclusive_minimum: impl Into<json_codec::Value>,
        minimum: impl Into<json_codec::Value>,
        expected: bool,
    ) {
        let schema = json!({
            "$schema": schema_uri,
            "minimum": minimum.into(),
            "exclusiveMinimum": exclusive_minimum.into()
        });

        let is_valid_result = crate::meta::is_valid(&schema);
        assert_eq!(is_valid_result, expected);

        let validate_result = crate::meta::validate(&schema);
        assert_eq!(validate_result.is_ok(), expected);
    }

    #[test]
    fn test_invalid_schema_uri() {
        let schema = json!({
            "$schema": "invalid-uri",
            "type": "string"
        });

        let result = crate::options().without_schema_validation().build(&schema);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Unknown meta-schema"));
        assert!(error.to_string().contains("invalid-uri"));
    }

    #[test]
    fn test_invalid_schema_keyword() {
        let schema = json!({
            // Note `htt`, not `http`
            "$schema": "htt://json-schema.org/draft-07/schema",
            "type": "string"
        });

        // Without registering the meta-schema, this should fail
        let result = crate::options().without_schema_validation().build(&schema);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Unknown meta-schema"));
        assert!(error
            .to_string()
            .contains("htt://json-schema.org/draft-07/schema"));
    }

    #[test_case(Draft::Draft4)]
    #[test_case(Draft::Draft6)]
    #[test_case(Draft::Draft7)]
    fn meta_schemas(draft: Draft) {
        // See GH-258
        for schema in [json!({"enum": [0, 0.0]}), json!({"enum": []})] {
            assert!(crate::options().with_draft(draft).build(&schema).is_ok());
        }
    }

    #[test]
    fn incomplete_escape_in_pattern() {
        // See GH-253
        let schema = json!({"pattern": "\\u"});
        assert!(crate::validator_for(&schema).is_err());
    }

    #[test]
    fn validation_error_propagation() {
        fn foo() -> Result<(), Box<dyn std::error::Error>> {
            let schema = json!({});
            let validator = validator_for(&schema)?;
            let _ = validator.is_valid(&json!({}));
            Ok(())
        }
        let _ = foo();
    }

    #[test]
    fn test_meta_validation_with_unknown_schema() {
        let schema = json!({
            "$schema": "json-schema:///custom",
            "type": "string"
        });

        // Meta-validation now errors when the meta-schema is unknown/unregistered
        assert!(crate::meta::validate(&schema).is_err());

        // Building a validator also fails without registration
        let result = crate::validator_for(&schema);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), feature = "resolve-file"))]
    fn test_meta_validation_respects_metaschema_draft() {
        use std::io::Write;

        let mut temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let meta_schema_draft7 = json!({
            "$id": "http://example.com/meta/draft7",
            "$schema": "http://json-schema.org/draft-07/schema",
            "type": ["object", "boolean"],
            "properties": {
                "$schema": { "type": "string" },
                "type": {},
                "properties": { "type": "object" }
            },
            "additionalProperties": false
        });
        write!(temp_file, "{meta_schema_draft7}").expect("Failed to write to temp file");

        let uri = crate::retriever::path_to_uri(temp_file.path());

        let schema_using_draft7_meta = json!({
            "$schema": uri,
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "unevaluatedProperties": false
        });

        let schema_valid_for_draft7_meta = json!({
            "$schema": uri,
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });

        assert!(crate::meta::is_valid(&meta_schema_draft7));
        assert!(!crate::meta::is_valid(&schema_using_draft7_meta));
        assert!(crate::meta::is_valid(&schema_valid_for_draft7_meta));
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), feature = "resolve-file"))]
    fn test_meta_schema_chain_resolution() {
        use std::io::Write;

        // Create intermediate meta-schema pointing to Draft 2020-12
        let mut intermediate_file =
            tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let intermediate_meta = json!({
            "$id": "http://example.com/meta/intermediate",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object"
        });
        write!(intermediate_file, "{intermediate_meta}").expect("Failed to write to temp file");
        let intermediate_uri = crate::retriever::path_to_uri(intermediate_file.path());

        // Create custom meta-schema with unknown draft that points to intermediate
        // This triggers the chain resolution code path in resolve_meta_schema_chain
        let mut custom_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let custom_meta = json!({
            "$id": "http://example.com/meta/custom",
            "$schema": intermediate_uri,
            "type": "object"
        });
        write!(custom_file, "{custom_meta}").expect("Failed to write to temp file");
        let custom_uri = crate::retriever::path_to_uri(custom_file.path());

        let schema = json!({
            "$schema": custom_uri,
            "type": "string"
        });

        // Should successfully resolve through the chain and detect Draft 2020-12
        assert!(crate::meta::is_valid(&schema));
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), feature = "resolve-file"))]
    fn test_circular_meta_schema_reference() {
        use std::io::Write;

        // Create meta-schema A pointing to meta-schema B
        let mut meta_a_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let meta_a_uri = crate::retriever::path_to_uri(meta_a_file.path());

        // Create meta-schema B pointing back to meta-schema A
        let mut meta_b_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let meta_b_uri = crate::retriever::path_to_uri(meta_b_file.path());

        let meta_a = json!({
            "$id": "http://example.com/meta/a",
            "$schema": &meta_b_uri,
            "type": "object"
        });
        write!(meta_a_file, "{meta_a}").expect("Failed to write to temp file");

        let meta_b = json!({
            "$id": "http://example.com/meta/b",
            "$schema": &meta_a_uri,
            "type": "object"
        });
        write!(meta_b_file, "{meta_b}").expect("Failed to write to temp file");

        let schema = json!({
            "$schema": meta_a_uri.clone(),
            "type": "string"
        });

        // Should return a circular meta-schema error
        let result = crate::meta::options().validate(&schema);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular meta-schema reference"));
    }

    #[test]
    fn simple_schema_with_unknown_draft() {
        // Define a custom meta-schema
        let meta_schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "http://custom.example.com/schema",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/applicator": true,
                "https://json-schema.org/draft/2020-12/vocab/validation": true,
            }
        });

        // Schema using the custom meta-schema
        let schema = json!({
            "$schema": "http://custom.example.com/schema",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });

        let registry = Registry::new()
            .add("http://custom.example.com/schema", meta_schema)
            .expect("Should accept meta-schema")
            .prepare()
            .expect("Should create registry");
        let validator = crate::options()
            .without_schema_validation()
            .with_registry(&registry)
            .build(&schema)
            .expect("Should build validator");

        // Valid instance
        assert!(validator.is_valid(&json!({"name": "test"})));

        // Invalid instance - name should be string, not number
        assert!(!validator.is_valid(&json!({"name": 123})));

        // Also verify type validation works
        assert!(!validator.is_valid(&json!("not an object")));
    }

    #[test]
    fn custom_meta_schema_support() {
        // Define a custom meta-schema that extends Draft 2020-12
        let meta_schema = json!({
            "$id": "http://example.com/meta/schema",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "Core schema definition",
            "type": "object",
            "allOf": [
                {
                    "$ref": "#/$defs/editable"
                },
                {
                    "$ref": "#/$defs/core"
                }
            ],
            "properties": {
                "properties": {
                    "type": "object",
                    "patternProperties": {
                        ".*": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": [
                                        "array",
                                        "boolean",
                                        "integer",
                                        "number",
                                        "object",
                                        "string",
                                        "null"
                                    ]
                                }
                            }
                        }
                    },
                    "propertyNames": {
                        "type": "string",
                        "pattern": "^[A-Za-z_][A-Za-z0-9_]*$"
                    }
                }
            },
            "unevaluatedProperties": false,
            "required": [
                "properties"
            ],
            "$defs": {
                "core": {
                    "type": "object",
                    "properties": {
                        "$id": {
                            "type": "string"
                        },
                        "$schema": {
                            "type": "string"
                        },
                        "type": {
                            "const": "object"
                        },
                        "title": {
                            "type": "string"
                        },
                        "description": {
                            "type": "string"
                        },
                        "additionalProperties": {
                            "type": "boolean",
                            "const": false
                        }
                    },
                    "required": [
                        "$id",
                        "$schema",
                        "type"
                    ]
                },
                "editable": {
                    "type": "object",
                    "properties": {
                        "creationDate": {
                            "type": "string",
                            "format": "date-time"
                        },
                        "updateDate": {
                            "type": "string",
                            "format": "date-time"
                        }
                    },
                    "required": [
                        "creationDate"
                    ]
                }
            }
        });

        // A schema that uses the custom meta-schema
        let element_schema = json!({
            "$schema": "http://example.com/meta/schema",
            "$id": "http://example.com/schemas/element",
            "title": "Element",
            "description": "An element",
            "creationDate": "2024-12-31T12:31:53+01:00",
            "properties": {
                "value": {
                    "type": "string"
                }
            },
            "type": "object"
        });

        let registry = Registry::new()
            .add("http://example.com/meta/schema", meta_schema)
            .expect("Should accept meta-schema")
            .prepare()
            .expect("Should create registry");
        let validator = crate::options()
            .without_schema_validation()
            .with_registry(&registry)
            .build(&element_schema)
            .expect("Should successfully build validator with custom meta-schema");

        let valid_instance = json!({
            "value": "test string"
        });
        assert!(validator.is_valid(&valid_instance));

        let invalid_instance = json!({
            "value": 123
        });
        assert!(!validator.is_valid(&invalid_instance));
    }

    #[test]
    fn custom_meta_schema_with_fragment_finds_vocabularies() {
        // Custom meta-schema URIs with trailing # should be found in registry
        let custom_meta = json!({
            "$id": "http://example.com/custom-with-unevaluated",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/applicator": true,
                "https://json-schema.org/draft/2020-12/vocab/validation": true,
                "https://json-schema.org/draft/2020-12/vocab/unevaluated": true
            }
        });

        let registry = Registry::new()
            .add("http://example.com/custom-with-unevaluated", custom_meta)
            .expect("Should accept meta-schema")
            .prepare()
            .expect("Should create registry");

        let schema = json!({
            "$id": "http://example.com/subject",
            "$schema": "http://example.com/custom-with-unevaluated#",
            "type": "object",
            "properties": {
                "foo": { "type": "string" }
            },
            "unevaluatedProperties": false
        });

        let validator = crate::options()
            .without_schema_validation()
            .with_registry(&registry)
            .build(&schema)
            .expect("Should build validator");

        assert!(validator.is_valid(&json!({"foo": "bar"})));
        assert!(!validator.is_valid(&json!({"foo": "bar", "extra": "value"})));
    }

    fn dialect_registry(vocabularies: &Value) -> Registry<'static> {
        Registry::new()
            .add(
                "https://example.com/dialect",
                json!({
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                    "$id": "https://example.com/dialect",
                    "$vocabulary": vocabularies,
                }),
            )
            .expect("Should accept meta-schema")
            .prepare()
            .expect("Should create registry")
    }

    fn format_assertion_dialect(required: bool) -> Value {
        json!({
            "https://json-schema.org/draft/2020-12/vocab/core": true,
            "https://json-schema.org/draft/2020-12/vocab/validation": true,
            "https://json-schema.org/draft/2020-12/vocab/format-assertion": required,
        })
    }

    // The boolean only steers implementations that do not know the vocabulary.
    #[test_case(true)]
    #[test_case(false)]
    fn format_assertion_vocabulary_asserts_without_opt_in(required: bool) {
        let registry = dialect_registry(&format_assertion_dialect(required));
        let validator = crate::options()
            .with_registry(&registry)
            .build(&json!({
                "$schema": "https://example.com/dialect",
                "type": "string",
                "format": "date-time"
            }))
            .expect("Should build validator");

        assert!(validator.is_valid(&json!("2026-08-22T12:00:00Z")));
        assert!(!validator.is_valid(&json!("not-a-date")));
    }

    #[test_case(false)]
    #[test_case(true)]
    fn format_assertion_vocabulary_rejects_unknown_format(ignore_unknown_formats: bool) {
        let registry = dialect_registry(&format_assertion_dialect(true));
        let error = crate::options()
            .with_registry(&registry)
            .should_ignore_unknown_formats(ignore_unknown_formats)
            .build(&json!({
                "$schema": "https://example.com/dialect",
                "format": "totally-made-up"
            }))
            .expect_err("Should reject unknown format");

        assert_eq!(
            error.to_string(),
            "Unknown format: 'totally-made-up'. The meta-schema asserts formats, so unrecognized ones cannot be ignored. Register a check for it or disable format validation"
        );
    }

    #[test]
    fn format_assertion_vocabulary_honors_explicit_opt_out() {
        let registry = dialect_registry(&format_assertion_dialect(true));
        let validator = crate::options()
            .with_registry(&registry)
            .should_validate_formats(false)
            .build(&json!({
                "$schema": "https://example.com/dialect",
                "format": "date-time"
            }))
            .expect("Should build validator");

        assert!(validator.is_valid(&json!("not-a-date")));
    }

    #[test_case(true, false ; "required")]
    #[test_case(false, true ; "optional")]
    fn draft201909_format_vocabulary_asserts_when_required(required: bool, expected: bool) {
        let registry = Registry::new()
            .add(
                "https://example.com/dialect",
                json!({
                    "$schema": "https://json-schema.org/draft/2019-09/schema",
                    "$id": "https://example.com/dialect",
                    "$vocabulary": {
                        "https://json-schema.org/draft/2019-09/vocab/core": true,
                        "https://json-schema.org/draft/2019-09/vocab/validation": true,
                        "https://json-schema.org/draft/2019-09/vocab/format": required,
                    },
                }),
            )
            .expect("Should accept meta-schema")
            .prepare()
            .expect("Should create registry");
        let validator = crate::options()
            .with_registry(&registry)
            .build(&json!({
                "$schema": "https://example.com/dialect",
                "format": "ipv4"
            }))
            .expect("Should build validator");

        assert_eq!(validator.is_valid(&json!("not-an-ipv4")), expected);
    }

    #[test]
    fn dialect_without_id_declares_vocabularies() {
        let registry = Registry::new()
            .add(
                "https://example.com/dialect",
                json!({
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                    "$vocabulary": format_assertion_dialect(true),
                }),
            )
            .expect("Should accept meta-schema")
            .prepare()
            .expect("Should create registry");
        let validator = crate::options()
            .with_registry(&registry)
            .build(&json!({
                "$schema": "https://example.com/dialect",
                "format": "ipv4"
            }))
            .expect("Should build validator");

        assert!(!validator.is_valid(&json!("not-an-ipv4")));
    }

    #[test]
    fn nested_dialect_declares_vocabularies() {
        let registry = dialect_registry(&format_assertion_dialect(true));
        let validator = crate::options()
            .with_draft(Draft::Draft4)
            .with_registry(&registry)
            .build(&json!({
                "properties": {
                    "address": {"$schema": "https://example.com/dialect", "format": "ipv4"}
                }
            }))
            .expect("Should build validator");

        assert!(!validator.is_valid(&json!({"address": "not-an-ipv4"})));
    }

    #[test_case("https://json-schema.org/draft/2020-12/meta/format-assertion" ; "2020-12 format-assertion")]
    #[test_case("https://json-schema.org/draft/2019-09/meta/format" ; "2019-09 format")]
    fn bundled_format_meta_schema_asserts(meta_schema: &str) {
        let validator = crate::options()
            .build(&json!({
                "$schema": meta_schema,
                "format": "ipv4"
            }))
            .expect("Should build validator");

        assert!(validator.is_valid(&json!("127.0.0.1")));
        assert!(!validator.is_valid(&json!("not-an-ipv4")));
    }

    #[test]
    fn format_annotation_vocabulary_does_not_assert() {
        let registry = dialect_registry(&json!({
            "https://json-schema.org/draft/2020-12/vocab/core": true,
            "https://json-schema.org/draft/2020-12/vocab/format-annotation": true,
        }));
        let validator = crate::options()
            .with_registry(&registry)
            .build(&json!({
                "$schema": "https://example.com/dialect",
                "format": "date-time"
            }))
            .expect("Should build validator");

        assert!(validator.is_valid(&json!("not-a-date")));
    }

    const UNKNOWN_VOCABULARY_ERROR: &str = "Unknown vocabulary: 'https://example.com/vocab/made-up' is required by the meta-schema. Adjust configuration to declare support for it";

    #[test_case("https://example.com/vocab/made-up", true, false, Some(UNKNOWN_VOCABULARY_ERROR) ; "required undeclared")]
    #[test_case("https://example.com/vocab/made-up", false, false, None ; "optional undeclared")]
    #[test_case("https://example.com/vocab/made-up", true, true, None ; "required declared")]
    #[test_case("https://Example.COM/vocab/made-up", true, true, None ; "declared uppercase host")]
    #[test_case("https://example.com/vocab/./made-up", true, true, None ; "declared dot segment")]
    #[test_case("https://example.com/vocab/made%2Dup", true, true, None ; "declared percent-encoded unreserved")]
    fn unknown_vocabulary(uri: &str, required: bool, declared: bool, expected: Option<&str>) {
        let registry = dialect_registry(&json!({
            "https://json-schema.org/draft/2020-12/vocab/core": true,
            uri: required,
        }));
        let mut options = crate::options().with_registry(&registry);
        if declared {
            options = options.with_vocabulary(uri);
        }

        // `Clone` keeps the declarations.
        let error = options
            .clone()
            .build(&json!({"$schema": "https://example.com/dialect"}))
            .err()
            .map(|error| error.to_string());

        assert_eq!(error.as_deref(), expected);
    }

    #[test_case("$ref", "https://example.com/inner" ; "ref to resource")]
    #[test_case("$ref", "https://example.com/inner#/$defs/leaf" ; "ref into resource")]
    #[test_case("$dynamicRef", "https://example.com/inner" ; "dynamic ref to resource")]
    fn required_unknown_vocabulary_is_rejected_behind_reference(keyword: &str, target: &str) {
        let registry = Registry::new()
            .add(
                "https://example.com/dialect",
                json!({
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                    "$id": "https://example.com/dialect",
                    "$vocabulary": {
                        "https://json-schema.org/draft/2020-12/vocab/core": true,
                        "https://example.com/vocab/made-up": true,
                    },
                }),
            )
            .expect("Should accept meta-schema")
            .add(
                "https://example.com/inner",
                json!({
                    "$schema": "https://example.com/dialect",
                    "$id": "https://example.com/inner",
                    "type": "string",
                    "$defs": {"leaf": {"type": "string"}},
                }),
            )
            .expect("Should accept inner schema")
            .prepare()
            .expect("Should create registry");
        let error = crate::options()
            .with_registry(&registry)
            .build(&json!({keyword: target}))
            .expect_err("Should reject required unknown vocabulary");

        assert_eq!(error.to_string(), UNKNOWN_VOCABULARY_ERROR);
    }

    #[test]
    fn strict_meta_schema_catches_typos() {
        // Issue #764: Use strict meta-schema with unevaluatedProperties: false
        // to catch typos in schema keywords

        let strict_meta = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://json-schema.org/draft/2020-12/strict",
            "$dynamicAnchor": "meta",
            "$ref": "https://json-schema.org/draft/2020-12/schema",
            "unevaluatedProperties": false
        });

        let registry = Registry::new()
            .add("https://json-schema.org/draft/2020-12/strict", strict_meta)
            .expect("Should accept strict meta-schema")
            .prepare()
            .expect("Should create registry");

        // Valid schema - all keywords are recognized
        let valid_schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/strict",
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 1}
            }
        });

        assert!(crate::meta::options()
            .with_registry(&registry)
            .is_valid(&valid_schema));

        // Invalid schema - top-level typo "typ" instead of "type"
        let invalid_schema_top_level = json!({
            "$schema": "https://json-schema.org/draft/2020-12/strict",
            "typ": "string"  // Typo
        });

        assert!(!crate::meta::options()
            .with_registry(&registry)
            .is_valid(&invalid_schema_top_level));

        // Invalid schema - nested invalid keyword "minSize" (not a real JSON Schema keyword)
        let invalid_schema_nested = json!({
            "$schema": "https://json-schema.org/draft/2020-12/strict",
            "type": "object",
            "properties": {
                "name": {"type": "string", "minSize": 1}  // Invalid keyword in nested schema
            }
        });

        assert!(!crate::meta::options()
            .with_registry(&registry)
            .is_valid(&invalid_schema_nested));
    }

    #[test]
    fn custom_meta_schema_preserves_underlying_draft_behavior() {
        // Regression test: Custom meta-schemas should preserve the draft-specific
        // behavior of their underlying draft, not default to Draft 2020-12
        // Draft 7 specific behavior: $ref siblings are ignored

        let custom_meta_draft7 = json!({
            "$id": "http://example.com/meta/draft7-custom",
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "customKeyword": {"type": "string"}
            }
        });

        let registry = Registry::new()
            .add("http://example.com/meta/draft7-custom", custom_meta_draft7)
            .expect("Should accept meta-schema")
            .prepare()
            .expect("Should create registry");

        let schema = json!({
            "$id": "http://example.com/subject",
            "$schema": "http://example.com/meta/draft7-custom",
            "$ref": "#/$defs/positiveNumber",
            "maximum": 5,
            "$defs": {
                "positiveNumber": {
                    "type": "number",
                    "minimum": 0
                }
            }
        });

        let validator = crate::options()
            .without_schema_validation()
            .with_registry(&registry)
            .build(&schema)
            .expect("Should build validator");

        // In Draft 7: siblings of $ref are ignored, so maximum: 5 has no effect
        // In Draft 2020-12: siblings are evaluated, so maximum: 5 would apply
        assert!(validator.is_valid(&json!(10)));
    }

    mod meta_options_tests {
        use super::*;
        use crate::Registry;

        #[test]
        fn test_meta_options_with_registry_valid_schema() {
            let custom_meta = json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "$schema": { "type": "string" },
                    "type": { "type": "string" },
                    "maxLength": { "type": "integer" }
                },
                "additionalProperties": false
            });

            let registry = Registry::new()
                .add("http://example.com/meta", custom_meta)
                .unwrap()
                .prepare()
                .unwrap();

            let schema = json!({
                "$schema": "http://example.com/meta",
                "type": "string",
                "maxLength": 10
            });

            assert!(crate::meta::options()
                .with_registry(&registry)
                .is_valid(&schema));

            assert!(crate::meta::options()
                .with_registry(&registry)
                .validate(&schema)
                .is_ok());
        }

        #[test]
        fn test_meta_options_with_registry_invalid_schema() {
            let custom_meta = json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "type": { "type": "string" }
                },
                "additionalProperties": false
            });

            let registry = Registry::new()
                .add("http://example.com/meta", custom_meta)
                .unwrap()
                .prepare()
                .unwrap();

            // Schema has disallowed property
            let schema = json!({
                "$schema": "http://example.com/meta",
                "type": "string",
                "maxLength": 10  // Not allowed by custom meta-schema
            });

            assert!(!crate::meta::options()
                .with_registry(&registry)
                .is_valid(&schema));

            assert!(crate::meta::options()
                .with_registry(&registry)
                .validate(&schema)
                .is_err());
        }

        #[test]
        fn test_meta_options_with_registry_chain() {
            // Create a chain: custom-meta -> draft2020-12
            let custom_meta = json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object"
            });

            let registry = Registry::new()
                .add("http://example.com/custom", custom_meta)
                .unwrap()
                .prepare()
                .unwrap();

            let schema = json!({
                "$schema": "http://example.com/custom",
                "type": "string"
            });

            assert!(crate::meta::options()
                .with_registry(&registry)
                .is_valid(&schema));
        }

        #[test]
        fn test_meta_options_with_registry_multi_level_chain() {
            // Create chain: schema -> meta-level-2 -> meta-level-1 -> draft2020-12
            let meta_level_1 = json!({
                "$id": "http://example.com/meta/level1",
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "customProp": { "type": "boolean" }
                }
            });

            let meta_level_2 = json!({
                "$id": "http://example.com/meta/level2",
                "$schema": "http://example.com/meta/level1",
                "type": "object",
                "customProp": true
            });

            let registry = Registry::new()
                .extend([
                    ("http://example.com/meta/level1", meta_level_1),
                    ("http://example.com/meta/level2", meta_level_2),
                ])
                .unwrap()
                .prepare()
                .unwrap();

            let schema = json!({
                "$schema": "http://example.com/meta/level2",
                "type": "string",
                "customProp": true
            });

            assert!(crate::meta::options()
                .with_registry(&registry)
                .is_valid(&schema));
        }

        #[test]
        fn test_meta_options_with_registry_multi_document_meta_schema() {
            let shared_constraints = json!({
                "$id": "http://example.com/meta/shared",
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "maxLength": { "type": "integer", "minimum": 0 }
                }
            });

            let root_meta = json!({
                "$id": "http://example.com/meta/root",
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "$schema": { "type": "string" },
                    "type": { "type": "string" }
                },
                "allOf": [
                    { "$ref": "http://example.com/meta/shared" }
                ]
            });

            let registry = Registry::new()
                .extend([
                    ("http://example.com/meta/root", root_meta),
                    ("http://example.com/meta/shared", shared_constraints),
                ])
                .unwrap()
                .prepare()
                .unwrap();

            let schema = json!({
                "$schema": "http://example.com/meta/root",
                "type": "string",
                "maxLength": 5
            });

            let result = crate::meta::options()
                .with_registry(&registry)
                .validate(&schema);

            assert!(
                result.is_ok(),
                "meta validation failed even though registry contains all meta-schemas: {}",
                result.unwrap_err()
            );

            assert!(crate::meta::options()
                .with_registry(&registry)
                .is_valid(&schema));
        }

        #[test]
        fn test_meta_options_without_registry_unknown_metaschema() {
            let schema = json!({
                "$schema": "http://0.0.0.0/nonexistent",
                "type": "string"
            });

            // Without registry, should fail to resolve
            let result = crate::meta::options().validate(&schema);
            assert!(result.is_err());
        }

        #[test]
        #[should_panic(expected = "Failed to resolve meta-schema")]
        fn test_meta_options_is_valid_panics_on_missing_metaschema() {
            let schema = json!({
                "$schema": "http://0.0.0.0/nonexistent",
                "type": "string"
            });

            // is_valid() should panic if meta-schema cannot be resolved
            let _ = crate::meta::options().is_valid(&schema);
        }

        #[test]
        fn test_meta_options_with_registry_missing_metaschema() {
            let custom_meta = json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object"
            });

            let registry = Registry::new()
                .add("http://example.com/meta1", custom_meta)
                .unwrap()
                .prepare()
                .unwrap();

            // Schema references a different meta-schema not in registry
            let schema = json!({
                "$schema": "http://example.com/meta2",
                "type": "string"
            });

            let result = crate::meta::options()
                .with_registry(&registry)
                .validate(&schema);

            assert!(result.is_err());
        }

        #[test]
        fn test_meta_options_circular_reference_detection() {
            // Create a circular reference: meta1 -> meta2 -> meta1
            let meta1 = json!({
                "$id": "http://example.com/meta1",
                "$schema": "http://example.com/meta2",
                "type": "object"
            });

            let meta2 = json!({
                "$id": "http://example.com/meta2",
                "$schema": "http://example.com/meta1",
                "type": "object"
            });

            let registry = Registry::new()
                .extend([
                    ("http://example.com/meta1", meta1),
                    ("http://example.com/meta2", meta2),
                ])
                .unwrap()
                .prepare()
                .unwrap();

            let schema = json!({
                "$schema": "http://example.com/meta1",
                "type": "string"
            });

            let result = crate::meta::options()
                .with_registry(&registry)
                .validate(&schema);

            assert!(result.is_err());
            // Check it's specifically a circular error
            let err = result.unwrap_err();
            assert!(err.to_string().contains("Circular"));
        }

        #[test]
        fn test_meta_options_standard_drafts_without_registry() {
            // Standard drafts should work without registry
            let schemas = vec![
                json!({ "$schema": "http://json-schema.org/draft-04/schema#", "type": "string" }),
                json!({ "$schema": "http://json-schema.org/draft-06/schema#", "type": "string" }),
                json!({ "$schema": "http://json-schema.org/draft-07/schema#", "type": "string" }),
                json!({ "$schema": "https://json-schema.org/draft/2019-09/schema", "type": "string" }),
                json!({ "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "string" }),
            ];

            for schema in schemas {
                assert!(
                    crate::meta::options().is_valid(&schema),
                    "Failed for schema: {schema}"
                );
            }
        }

        #[test]
        fn test_meta_options_validate_returns_specific_errors() {
            let custom_meta = json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "required": ["type"]
            });

            let registry = Registry::new()
                .add("http://example.com/meta", custom_meta)
                .unwrap()
                .prepare()
                .unwrap();

            // Schema missing required property
            let schema = json!({
                "$schema": "http://example.com/meta",
                "properties": {
                    "name": { "type": "string" }
                }
            });

            let result = crate::meta::options()
                .with_registry(&registry)
                .validate(&schema);

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("required") || err.to_string().contains("type"));
        }

        #[test]
        fn test_meta_options_builds_validator_with_resolved_draft() {
            let custom_meta = json!({
                "$id": "http://example.com/meta/draft7-based",
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "$schema": { "type": "string" },
                    "type": { "type": "string" },
                    "minLength": { "type": "integer" }
                },
                "additionalProperties": false
            });

            let registry = Registry::new()
                .add("http://example.com/meta/draft7-based", custom_meta)
                .unwrap()
                .prepare()
                .unwrap();

            let schema = json!({
                "$schema": "http://example.com/meta/draft7-based",
                "type": "string",
                "minLength": 5
            });

            let result = crate::meta::options()
                .with_registry(&registry)
                .validate(&schema);

            assert!(result.is_ok());
        }

        #[test]
        fn test_meta_options_validator_uses_correct_draft() {
            let custom_meta_draft6 = json!({
                "$id": "http://example.com/meta/draft6-based",
                "$schema": "http://json-schema.org/draft-06/schema#",
                "type": "object",
                "properties": {
                    "$schema": { "type": "string" },
                    "type": { "type": "string" },
                    "exclusiveMinimum": { "type": "number" }
                },
                "additionalProperties": false
            });

            let registry = Registry::new()
                .add("http://example.com/meta/draft6-based", custom_meta_draft6)
                .unwrap()
                .prepare()
                .unwrap();

            let schema_valid_for_draft6 = json!({
                "$schema": "http://example.com/meta/draft6-based",
                "type": "number",
                "exclusiveMinimum": 0
            });

            let result = crate::meta::options()
                .with_registry(&registry)
                .validate(&schema_valid_for_draft6);

            assert!(result.is_ok());
        }

        #[test]
        fn test_meta_options_without_schema_validation_in_built_validator() {
            let custom_meta = json!({
                "$id": "http://example.com/meta/custom",
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "$schema": { "type": "string" },
                    "type": { "type": "string" }
                },
                "additionalProperties": false
            });

            let registry = Registry::new()
                .add("http://example.com/meta/custom", custom_meta)
                .unwrap()
                .prepare()
                .unwrap();

            let schema = json!({
                "$schema": "http://example.com/meta/custom",
                "type": "string"
            });

            let result = crate::meta::options()
                .with_registry(&registry)
                .validate(&schema);

            assert!(result.is_ok());
        }

        #[test]
        fn test_meta_validation_uses_resolved_draft_from_chain() {
            // Chain: user-schema -> custom-meta -> Draft 4
            // Validator should use Draft 4 rules to validate the schema
            let custom_meta = json!({
                "$id": "http://example.com/meta/draft4-based",
                "$schema": "http://json-schema.org/draft-04/schema#",
                "type": "object",
                "properties": {
                    "$schema": { "type": "string" },
                    "type": { "type": "string" },
                    "enum": { "type": "array" },
                    "const": { "type": "string" }
                },
                "additionalProperties": false
            });

            let registry = Registry::new()
                .add("http://example.com/meta/draft4-based", custom_meta)
                .unwrap()
                .prepare()
                .unwrap();

            let schema = json!({
                "$schema": "http://example.com/meta/draft4-based",
                "type": "string",
                "const": "foo"
            });

            let result = crate::meta::options()
                .with_registry(&registry)
                .validate(&schema);

            assert!(result.is_ok());
        }

        #[test]
        fn test_meta_validation_multi_level_chain_uses_resolved_draft() {
            // Multi-level chain: user-schema -> meta-2 -> meta-1 -> Draft 4
            let meta_level_1 = json!({
                "$id": "http://example.com/meta/level1",
                "$schema": "http://json-schema.org/draft-04/schema#",
                "type": "object",
                "properties": {
                    "customKeyword": { "type": "boolean" }
                }
            });

            let meta_level_2 = json!({
                "$id": "http://example.com/meta/level2",
                "$schema": "http://example.com/meta/level1",
                "type": "object",
                "properties": {
                    "$schema": { "type": "string" },
                    "type": { "type": "string" },
                    "minimum": { "type": "number" },
                    "exclusiveMinimum": { "type": "boolean" }
                },
                "customKeyword": true,
                "additionalProperties": false
            });

            let registry = Registry::new()
                .extend([
                    ("http://example.com/meta/level1", meta_level_1),
                    ("http://example.com/meta/level2", meta_level_2),
                ])
                .unwrap()
                .prepare()
                .unwrap();

            let schema = json!({
                "$schema": "http://example.com/meta/level2",
                "type": "number",
                "minimum": 5,
                "exclusiveMinimum": true
            });

            let result = crate::meta::options()
                .with_registry(&registry)
                .validate(&schema);

            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_meta_validator_for_valid_schema() {
        let schema = json!({
            "type": "string",
            "maxLength": 5
        });

        let validator = crate::meta::validator_for(&schema).expect("Valid meta-schema");
        assert!(validator.is_valid(&schema));
    }

    #[test]
    fn test_meta_validator_for_invalid_schema() {
        let schema = json!({
            "type": "invalid_type"
        });

        let validator = crate::meta::validator_for(&schema).expect("Valid meta-schema");
        assert!(!validator.is_valid(&schema));
    }

    #[test]
    fn test_meta_validator_for_evaluate_api() {
        let schema = json!({
            "type": "string",
            "maxLength": 5
        });

        let validator = crate::meta::validator_for(&schema).expect("Valid meta-schema");
        let evaluation = validator.evaluate(&schema);

        let flag = evaluation.flag();
        assert!(flag.valid);
    }

    #[test]
    fn test_meta_validator_for_evaluate_api_invalid() {
        let schema = json!({
            "type": "invalid_type",
            "minimum": "not a number"
        });

        let validator = crate::meta::validator_for(&schema).expect("Valid meta-schema");
        let evaluation = validator.evaluate(&schema);

        let flag = evaluation.flag();
        assert!(!flag.valid);
    }

    #[test]
    fn test_meta_validator_for_all_drafts() {
        let schemas = vec![
            json!({ "$schema": "http://json-schema.org/draft-04/schema#", "type": "string" }),
            json!({ "$schema": "http://json-schema.org/draft-06/schema#", "type": "string" }),
            json!({ "$schema": "http://json-schema.org/draft-07/schema#", "type": "string" }),
            json!({ "$schema": "https://json-schema.org/draft/2019-09/schema", "type": "string" }),
            json!({ "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "string" }),
        ];

        for schema in schemas {
            let validator = crate::meta::validator_for(&schema).unwrap();
            assert!(validator.is_valid(&schema));
        }
    }

    #[test]
    fn test_meta_validator_for_iter_errors() {
        let schema = json!({
            "type": "invalid_type",
            "minimum": "not a number"
        });

        let validator = crate::meta::validator_for(&schema).expect("Valid meta-schema");
        let errors: Vec<_> = validator.iter_errors(&schema).collect();
        assert!(!errors.is_empty());
    }

    #[test_case(
        "http://json-schema.org/draft-04/schema#",
        "http://json-schema.org/draft-04/schema#/definitions/positiveInteger/minimum",
        "/definitions/positiveInteger/minimum";
        "draft 4"
    )]
    #[test_case(
        "http://json-schema.org/draft-06/schema#",
        "http://json-schema.org/draft-06/schema#/definitions/nonNegativeInteger/minimum",
        "/definitions/nonNegativeInteger/minimum";
        "draft 6"
    )]
    #[test_case(
        "http://json-schema.org/draft-07/schema#",
        "http://json-schema.org/draft-07/schema#/definitions/nonNegativeInteger/minimum",
        "/definitions/nonNegativeInteger/minimum";
        "draft 7"
    )]
    #[test_case(
        "https://json-schema.org/draft/2019-09/schema",
        "https://json-schema.org/draft/2019-09/meta/validation#/$defs/nonNegativeInteger/minimum",
        "/$defs/nonNegativeInteger/minimum";
        "draft 2019-09"
    )]
    #[test_case(
        "https://json-schema.org/draft/2020-12/schema",
        "https://json-schema.org/draft/2020-12/meta/validation#/$defs/nonNegativeInteger/minimum",
        "/$defs/nonNegativeInteger/minimum";
        "draft 2020-12"
    )]
    fn test_meta_validator_for_reports_negative_length(
        dialect: &str,
        schema_location: &str,
        schema_path: &str,
    ) {
        const NEGATIVE: &str = "-1 is less than the minimum of 0";
        let schema = json!({"$schema": dialect, "minLength": -1});
        let validator = crate::meta::validator_for(&schema).expect("Valid meta-schema");

        let evaluation = validator.evaluate(&schema);
        let evaluated: Vec<_> = evaluation
            .iter_errors()
            .map(|entry| {
                (
                    entry.instance_location.as_str().to_string(),
                    entry.schema_location.to_string(),
                    entry.error.to_string(),
                )
            })
            .collect();
        let errors: Vec<_> = validator
            .iter_errors(&schema)
            .map(|error| {
                (
                    error.to_string(),
                    error.instance_path().to_string(),
                    error.schema_path().to_string(),
                )
            })
            .collect();
        assert_eq!(
            (evaluation.flag().valid, evaluated, errors),
            (
                false,
                vec![(
                    "/minLength".to_string(),
                    schema_location.to_string(),
                    NEGATIVE.to_string()
                )],
                vec![(
                    NEGATIVE.to_string(),
                    "/minLength".to_string(),
                    schema_path.to_string()
                )]
            )
        );
    }
}
