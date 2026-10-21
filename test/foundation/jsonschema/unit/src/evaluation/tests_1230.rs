#[cfg(test)]
mod tests {
    use super::*;
    use json_codec::{json, Value};
    use std::sync::Arc;
    use test_case::test_case;

    fn loc() -> Location {
        Location::new()
    }

    fn annotation(value: json_codec::Value) -> Annotations {
        Annotations::new(value)
    }

    impl ErrorDescription {
        fn from_string(s: &str) -> Self {
            ErrorDescription {
                keyword: "error".into(),
                message: s.to_string(),
            }
        }
    }

    fn leaf_with_annotation(schema: &str, ann: json_codec::Value) -> EvaluationNode {
        EvaluationNode::valid(
            loc(),
            None,
            schema.to_string(),
            loc(),
            Some(annotation(ann)),
            ChildList::default(),
        )
    }

    fn leaf_with_error(schema: &str, msg: &str) -> EvaluationNode {
        EvaluationNode::invalid(
            loc(),
            None,
            schema.to_string(),
            loc(),
            None,
            vec![ErrorDescription::from_string(msg)],
            ChildList::default(),
        )
    }

    #[test]
    fn iter_annotations_visits_all_nodes() {
        let mut arena = EvaluationArena::default();
        let child = leaf_with_annotation("/child", json!({"k": "v"}));
        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            Some(annotation(json!({"root": true}))),
            ChildList::from_nodes(&mut arena, vec![child]),
        );
        let evaluation = Evaluation::with_root(arena, root);
        let entries: Vec<_> = evaluation.iter_annotations().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].schema_location, "/root");
        assert_eq!(entries[1].schema_location, "/child");
    }

    #[test]
    fn iter_errors_visits_all_nodes() {
        let mut arena = EvaluationArena::default();
        let child = leaf_with_error("/child", "boom");
        let root = EvaluationNode::invalid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            None,
            vec![ErrorDescription::from_string("root error")],
            ChildList::from_nodes(&mut arena, vec![child]),
        );
        let evaluation = Evaluation::with_root(arena, root);
        let entries: Vec<_> = evaluation.iter_errors().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].error.to_string(), "root error");
        assert_eq!(entries[1].error.to_string(), "boom");
    }

    #[test]
    fn flag_output_valid() {
        let arena = EvaluationArena::default();
        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            None,
            ChildList::default(),
        );
        let evaluation = Evaluation::with_root(arena, root);
        let flag = evaluation.flag();
        assert!(flag.valid);
    }

    #[test]
    fn flag_output_invalid() {
        let arena = EvaluationArena::default();
        let root = EvaluationNode::invalid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            None,
            vec![ErrorDescription::from_string("error")],
            ChildList::default(),
        );
        let evaluation = Evaluation::with_root(arena, root);
        let flag = evaluation.flag();
        assert!(!flag.valid);
    }

    #[test]
    fn flag_output_serialization() {
        let arena = EvaluationArena::default();
        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            None,
            ChildList::default(),
        );
        let evaluation = Evaluation::with_root(arena, root);
        let flag = evaluation.flag();
        let serialized = json_codec::to_value(flag).expect("serialization succeeds");
        assert_eq!(serialized, json!({"valid": true}));
    }

    #[test]
    fn list_output_serialization_valid() {
        let arena = EvaluationArena::default();
        let root = EvaluationNode::valid(
            loc(),
            None,
            "#".to_string(),
            loc(),
            None,
            ChildList::default(),
        );
        let evaluation = Evaluation::with_root(arena, root);
        let list = evaluation.list();
        let serialized = json_codec::to_value(list).expect("serialization succeeds");
        assert_eq!(
            serialized,
            json!({
                "valid": true,
                "details": [
                    {
                        "valid": true,
                        "evaluationPath": "",
                        "schemaLocation": "#",
                        "instanceLocation": ""
                    }
                ]
            })
        );
    }

    #[test]
    fn list_output_serialization_with_children() {
        let mut arena = EvaluationArena::default();
        let child1 = leaf_with_annotation("/child1", json!({"key": "value"}));
        let child2 = leaf_with_error("/child2", "child error");
        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            Some(annotation(json!({"root": true}))),
            ChildList::from_nodes(&mut arena, vec![child1, child2]),
        );
        let evaluation = Evaluation::with_root(arena, root);
        let list = evaluation.list();
        let serialized = json_codec::to_value(list).expect("serialization succeeds");
        assert_eq!(
            serialized,
            json!({
                "valid": true,
                "details": [
                    {
                        "valid": true,
                        "evaluationPath": "",
                        "schemaLocation": "/root",
                        "instanceLocation": "",
                        "annotations": {"root": true}
                    },
                    {
                        "valid": true,
                        "evaluationPath": "",
                        "schemaLocation": "/child1",
                        "instanceLocation": "",
                        "annotations": {"key": "value"}
                    },
                    {
                        "valid": false,
                        "evaluationPath": "",
                        "schemaLocation": "/child2",
                        "instanceLocation": "",
                        "errors": {"error": "child error"}
                    }
                ]
            })
        );
    }

    #[test]
    fn hierarchical_output_serialization() {
        let mut arena = EvaluationArena::default();
        let child = leaf_with_annotation("/child", json!({"nested": "data"}));
        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            Some(annotation(json!({"root": "annotation"}))),
            ChildList::from_nodes(&mut arena, vec![child]),
        );
        let evaluation = Evaluation::with_root(arena, root);
        let hierarchical = evaluation.hierarchical();
        let serialized = json_codec::to_value(hierarchical).expect("serialization succeeds");
        assert_eq!(
            serialized,
            json!({
                "valid": true,
                "evaluationPath": "",
                "schemaLocation": "/root",
                "instanceLocation": "",
                "annotations": {"root": "annotation"},
                "details": [
                    {
                        "valid": true,
                        "evaluationPath": "",
                        "schemaLocation": "/child",
                        "instanceLocation": "",
                        "annotations": {"nested": "data"}
                    }
                ]
            })
        );
    }

    #[test]
    fn outputs_include_errors_and_dropped_annotations() {
        let mut arena = EvaluationArena::default();
        let invalid_child = EvaluationNode::invalid(
            loc(),
            None,
            "/items/type".to_string(),
            Location::new().join(1usize),
            None,
            vec![ErrorDescription::from_string("child error")],
            ChildList::default(),
        );
        let prefix_child = leaf_with_annotation("/prefix", json!(0));
        let root = EvaluationNode::invalid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            Some(annotation(json!({"dropped": true}))),
            vec![ErrorDescription::from_string("root failure")],
            ChildList::from_nodes(&mut arena, vec![invalid_child, prefix_child]),
        );
        let evaluation = Evaluation::with_root(arena, root);
        let list = json_codec::to_value(evaluation.list()).expect("serialization succeeds");
        assert_eq!(
            list,
            json!({
                "valid": false,
                "details": [
                    {
                        "valid": false,
                        "evaluationPath": "",
                        "schemaLocation": "/root",
                        "instanceLocation": "",
                        "droppedAnnotations": {"dropped": true},
                        "errors": {"error": "root failure"}
                    },
                    {
                        "valid": false,
                        "evaluationPath": "",
                        "schemaLocation": "/items/type",
                        "instanceLocation": "/1",
                        "errors": {"error": "child error"}
                    },
                    {
                        "valid": true,
                        "evaluationPath": "",
                        "schemaLocation": "/prefix",
                        "instanceLocation": "",
                        "annotations": 0
                    }
                ]
            })
        );
        let hierarchical =
            json_codec::to_value(evaluation.hierarchical()).expect("serialization succeeds");
        assert_eq!(
            hierarchical,
            json!({
                "valid": false,
                "evaluationPath": "",
                "schemaLocation": "/root",
                "instanceLocation": "",
                "droppedAnnotations": {"dropped": true},
                "errors": {"error": "root failure"},
                "details": [
                    {
                        "valid": false,
                        "evaluationPath": "",
                        "schemaLocation": "/items/type",
                        "instanceLocation": "/1",
                        "errors": {"error": "child error"}
                    },
                    {
                        "valid": true,
                        "evaluationPath": "",
                        "schemaLocation": "/prefix",
                        "instanceLocation": "",
                        "annotations": 0
                    }
                ]
            })
        );
    }

    #[test]
    fn empty_evaluation_tree() {
        let arena = EvaluationArena::default();
        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            None,
            ChildList::default(),
        );
        let evaluation = Evaluation::with_root(arena, root);

        // No annotations
        assert_eq!(evaluation.iter_annotations().count(), 0);
        // No errors
        assert_eq!(evaluation.iter_errors().count(), 0);

        let flag = evaluation.flag();
        assert!(flag.valid);
    }

    #[test]
    fn deep_nesting() {
        let mut arena = EvaluationArena::default();
        // Create a deeply nested tree: root -> level1 -> level2 -> level3
        let level3 = leaf_with_annotation("/level3", json!({"level": 3}));
        let level2 = EvaluationNode::valid(
            loc(),
            None,
            "/level2".to_string(),
            loc(),
            Some(annotation(json!({"level": 2}))),
            ChildList::from_nodes(&mut arena, vec![level3]),
        );
        let level1 = EvaluationNode::valid(
            loc(),
            None,
            "/level1".to_string(),
            loc(),
            Some(annotation(json!({"level": 1}))),
            ChildList::from_nodes(&mut arena, vec![level2]),
        );
        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            Some(annotation(json!({"level": 0}))),
            ChildList::from_nodes(&mut arena, vec![level1]),
        );

        let evaluation = Evaluation::with_root(arena, root);
        let annotations: Vec<_> = evaluation.iter_annotations().collect();
        assert_eq!(annotations.len(), 4);

        // Check depth-first order
        assert_eq!(annotations[0].schema_location, "/root");
        assert_eq!(annotations[1].schema_location, "/level1");
        assert_eq!(annotations[2].schema_location, "/level2");
        assert_eq!(annotations[3].schema_location, "/level3");
    }

    #[test]
    fn wide_tree() {
        let mut arena = EvaluationArena::default();
        // Create a wide tree with many siblings
        let children = ChildList::from_nodes(
            &mut arena,
            (0..10).map(|i| leaf_with_annotation(&format!("/child{i}"), json!({"index": i}))),
        );

        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            Some(annotation(json!({"root": true}))),
            children,
        );

        let evaluation = Evaluation::with_root(arena, root);
        let annotations: Vec<_> = evaluation.iter_annotations().collect();
        assert_eq!(annotations.len(), 11); // root + 10 children
    }

    #[test]
    fn multiple_errors_per_node() {
        let arena = EvaluationArena::default();
        let errors = vec![
            ErrorDescription::from_string("error 1"),
            ErrorDescription::from_string("error 2"),
            ErrorDescription::from_string("error 3"),
        ];
        let root = EvaluationNode::invalid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            None,
            errors,
            ChildList::default(),
        );

        let evaluation = Evaluation::with_root(arena, root);
        let error_entries: Vec<_> = evaluation.iter_errors().collect();
        assert_eq!(error_entries.len(), 3);
        assert_eq!(error_entries[0].error.to_string(), "error 1");
        assert_eq!(error_entries[1].error.to_string(), "error 2");
        assert_eq!(error_entries[2].error.to_string(), "error 3");
    }

    #[test]
    fn mixed_valid_and_invalid_nodes() {
        let mut arena = EvaluationArena::default();
        let valid_child = leaf_with_annotation("/valid", json!({"ok": true}));
        let invalid_child = leaf_with_error("/invalid", "failed");

        let root = EvaluationNode::invalid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            Some(annotation(json!({"attempted": true}))),
            vec![ErrorDescription::from_string("root failed")],
            ChildList::from_nodes(&mut arena, vec![valid_child, invalid_child]),
        );

        let evaluation = Evaluation::with_root(arena, root);

        // The failing root drops the annotations of its valid child too
        assert_eq!(evaluation.iter_annotations().count(), 0);

        // Should have 2 errors (root + invalid child)
        let errors: Vec<_> = evaluation.iter_errors().collect();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn annotations_iterator_skips_nodes_without_annotations() {
        let mut arena = EvaluationArena::default();
        let no_annotation = EvaluationNode::valid(
            loc(),
            None,
            "/no_ann".to_string(),
            loc(),
            None,
            ChildList::default(),
        );
        let with_annotation = leaf_with_annotation("/with_ann", json!({"present": true}));

        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            None,
            ChildList::from_nodes(&mut arena, vec![no_annotation, with_annotation]),
        );

        let evaluation = Evaluation::with_root(arena, root);
        let annotations: Vec<_> = evaluation.iter_annotations().collect();
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].schema_location, "/with_ann");
    }

    #[test]
    fn errors_iterator_skips_nodes_without_errors() {
        let mut arena = EvaluationArena::default();
        let no_error = EvaluationNode::valid(
            loc(),
            None,
            "/no_error".to_string(),
            loc(),
            Some(annotation(json!({"ok": true}))),
            ChildList::default(),
        );
        let with_error = leaf_with_error("/with_error", "failed");

        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            None,
            ChildList::from_nodes(&mut arena, vec![no_error, with_error]),
        );

        let evaluation = Evaluation::with_root(arena, root);
        let errors: Vec<_> = evaluation.iter_errors().collect();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].schema_location, "/with_error");
    }

    #[test]
    fn error_entries_serialization_empty() {
        let entries = ErrorEntriesSerializer(&[]);
        let serialized = json_codec::to_value(&entries).expect("serialization succeeds");
        assert!(serialized.is_object());
        assert_eq!(serialized.as_object().unwrap().len(), 0);
    }

    #[test]
    fn error_entries_serialization_single() {
        let errors = vec![ErrorDescription::from_string("test error")];
        let entries = ErrorEntriesSerializer(&errors);
        let serialized = json_codec::to_value(&entries).expect("serialization succeeds");
        assert!(serialized.is_object());
        assert_eq!(serialized.as_object().unwrap().len(), 1);
        assert!(serialized.get("error").is_some());
    }

    #[test]
    fn error_entries_serialization_multiple() {
        let errors = vec![
            ErrorDescription::new("alpha", "error 1".to_string()),
            ErrorDescription::new("beta", "error 2".to_string()),
            ErrorDescription::new("gamma", "error 3".to_string()),
        ];
        let entries = ErrorEntriesSerializer(&errors);
        let serialized = json_codec::to_value(&entries).expect("serialization succeeds");
        assert_eq!(serialized.as_object().unwrap().len(), 3);
        assert!(serialized.get("alpha").is_some());
        assert!(serialized.get("beta").is_some());
        assert!(serialized.get("gamma").is_some());
    }

    #[test]
    fn error_entries_serialization_preserves_duplicates() {
        let errors = vec![
            ErrorDescription::new("required", "\"foo\" is required".to_string()),
            ErrorDescription::new("required", "\"bar\" is required".to_string()),
        ];
        let entries = ErrorEntriesSerializer(&errors);
        let serialized = json_codec::to_value(&entries).expect("serialization succeeds");
        let value = serialized
            .get("required")
            .expect("required keyword present")
            .as_array()
            .expect("multiple errors serialized as array");
        assert_eq!(value.len(), 2);
        assert_eq!(value[0], "\"foo\" is required");
        assert_eq!(value[1], "\"bar\" is required");
    }

    #[test]
    fn list_output_preserves_multiple_errors_per_keyword() {
        let arena = EvaluationArena::default();
        let errors = vec![
            ErrorDescription::new("required", "\"foo\" is required".to_string()),
            ErrorDescription::new("required", "\"bar\" is required".to_string()),
        ];
        let root = EvaluationNode::invalid(
            loc(),
            None,
            "/required".to_string(),
            loc(),
            None,
            errors,
            ChildList::default(),
        );

        let evaluation = Evaluation::with_root(arena, root);
        let list = json_codec::to_value(evaluation.list()).expect("serialization succeeds");
        let root_unit = list
            .get("details")
            .and_then(|value| value.as_array())
            .and_then(|details| details.first())
            .expect("list output contains root unit");
        let errors = root_unit
            .get("errors")
            .and_then(|errors| errors.get("required"))
            .and_then(|value| value.as_array())
            .expect("errors serialized as array");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0], "\"foo\" is required");
        assert_eq!(errors[1], "\"bar\" is required");
    }

    #[test]
    fn format_keyword_location_without_absolute() {
        let location = Location::new().join("properties").join("name");
        let formatted = format_keyword_location(&location, None);
        assert_eq!(formatted.as_ref(), "/properties/name");
    }

    #[test]
    fn format_keyword_location_keeps_encoded_fragment() {
        let location = Location::new().join("properties").join("s p");
        let uri = Arc::new(
            Uri::parse("http://example.com/schema.json#/properties/s%20p")
                .unwrap()
                .to_owned(),
        );
        let formatted = format_keyword_location(&location, Some(&uri));
        assert_eq!(
            formatted.as_ref(),
            "http://example.com/schema.json#/properties/s%20p"
        );
    }

    #[test]
    fn dropped_annotations_on_invalid_node() {
        let annotations = Some(annotation(json!({"dropped": true})));
        let root = EvaluationNode::invalid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            annotations.clone(),
            vec![ErrorDescription::from_string("failed")],
            ChildList::default(),
        );

        assert!(!root.valid);
        assert!(root.annotations.is_none());
        assert!(root.dropped_annotations.is_some());
        assert_eq!(
            root.dropped_annotations.as_ref().unwrap(),
            annotations.as_ref().unwrap()
        );
    }

    #[test]
    fn valid_node_has_no_dropped_annotations() {
        let annotations = Some(annotation(json!({"kept": true})));
        let root = EvaluationNode::valid(
            loc(),
            None,
            "/root".to_string(),
            loc(),
            annotations.clone(),
            ChildList::default(),
        );

        assert!(root.valid);
        assert!(root.annotations.is_some());
        assert!(root.dropped_annotations.is_none());
        assert_eq!(
            root.annotations.as_ref().unwrap(),
            annotations.as_ref().unwrap()
        );
    }

    #[test]
    fn absolute_keyword_location_populated_with_id() {
        use json_codec::json;

        // Schema with $id should populate absoluteKeywordLocation
        let schema = json!({
            "$id": "https://example.com/schema",
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let validator = crate::validator_for(&schema).expect("schema compiles");
        let evaluation = validator.evaluate(&json!({"name": "test"}));

        // Verify that absoluteKeywordLocation is populated for nodes
        let annotations: Vec<_> = evaluation.iter_annotations().collect();
        assert!(!annotations.is_empty());

        // At least one annotation should have an absolute keyword location
        let with_absolute = annotations
            .iter()
            .filter(|a| a.absolute_keyword_location.is_some())
            .count();

        assert!(with_absolute > 0);

        // Verify the absolute locations start with the schema's $id
        for annotation in annotations
            .iter()
            .filter(|a| a.absolute_keyword_location.is_some())
        {
            let uri_str = annotation.absolute_keyword_location.unwrap().as_str();
            assert!(uri_str.starts_with("https://example.com/schema"));
        }
    }

    #[test]
    fn annotations_value_returns_reference() {
        let expected = json!({"key": "value"});
        let annotations = Annotations::new(expected.clone());

        // value() should return a reference to the inner value
        assert_eq!(annotations.value(), &expected);
    }

    #[test]
    fn annotations_into_inner_consumes_and_returns_value() {
        let expected = json!({"key": "value", "nested": {"array": [1, 2, 3]}});
        let annotations = Annotations::new(expected.clone());

        // into_inner() should consume self and return the owned value
        let inner = annotations.into_inner();
        assert_eq!(inner, expected);
    }

    #[test]
    fn error_description_into_inner_consumes_and_returns_message() {
        let expected_message = "test error message";
        let error = ErrorDescription::from_string(expected_message);

        // into_inner() should consume self and return the owned message
        let message = error.into_inner();
        assert_eq!(message, expected_message);
    }

    #[test_case(json!(42), true)]
    #[test_case(json!("not a number"), false)]
    #[allow(clippy::needless_pass_by_value)]
    fn test_evaluation_is_valid(instance: Value, expected: bool) {
        let validator = crate::validator_for(&json!({"type": "number"})).expect("valid schema");
        assert_eq!(validator.evaluate(&instance).is_valid(), expected);
    }

    // A failing schema produces no annotations, including those of its subschemas
    #[test_case(
        json!({"properties": {"a": {"title": "x"}}, "required": ["b"]}),
        json!({"a": 1}),
        vec![];
        "invalid root"
    )]
    #[test_case(
        json!({"properties": {"obj": {"properties": {"a": {"title": "x"}}, "required": ["b"]}}}),
        json!({"obj": {"a": 1}}),
        vec![];
        "invalid nested object"
    )]
    #[test_case(
        json!({"properties": {"a": {"title": "x"}}, "required": ["a"]}),
        json!({"a": 1}),
        vec![
            ("/properties", "", json!(["a"])),
            ("/properties/a", "/a", json!({"title": "x"})),
        ];
        "valid root"
    )]
    #[test_case(
        json!({"anyOf": [{"title": "t0", "required": ["b"]}, {"title": "t1"}]}),
        json!({"a": 1}),
        vec![("/anyOf/1", "", json!({"title": "t1"}))];
        "valid anyOf branch"
    )]
    #[allow(clippy::needless_pass_by_value)]
    fn test_iter_annotations(schema: Value, instance: Value, expected: Vec<(&str, &str, Value)>) {
        let validator = crate::validator_for(&schema).expect("valid schema");
        let evaluation = validator.evaluate(&instance);
        let annotations: Vec<_> = evaluation
            .iter_annotations()
            .map(|entry| {
                (
                    entry.schema_location,
                    entry.instance_location.as_str(),
                    entry.annotations.value().clone(),
                )
            })
            .collect();
        assert_eq!(annotations, expected);
    }

    #[test]
    fn test_error_entry_display() {
        let schema = json!({
            "type": "object",
            "properties": {"age": {"type": "number"}},
            "required": ["name"]
        });
        let validator = crate::validator_for(&schema).expect("valid schema");
        let evaluation = validator.evaluate(&json!({"age": "oops"}));
        let rendered: Vec<String> = evaluation.iter_errors().map(|e| e.to_string()).collect();
        assert_eq!(
            rendered,
            vec![
                "\"name\" is a required property".to_string(),
                "/age: \"oops\" is not of type \"number\"".to_string(),
            ]
        );
    }

    fn node(evaluation_path: &str, schema_location: &str) -> Value {
        json!({
            "valid": true,
            "evaluationPath": evaluation_path,
            "schemaLocation": schema_location,
            "instanceLocation": ""
        })
    }

    fn with_details(mut node: Value, details: Vec<Value>) -> Value {
        node["details"] = Value::Array(details);
        node
    }

    fn outputs(schema: &Value) -> (Value, Value, usize) {
        let validator = crate::validator_for(schema).expect("valid schema");
        let evaluation = validator.evaluate(&json!("x"));
        (
            json_codec::to_value(evaluation.list()).expect("serialization succeeds"),
            json_codec::to_value(evaluation.hierarchical()).expect("serialization succeeds"),
            evaluation.iter_annotations().count(),
        )
    }

    // Drafts 4-7 ignore every `$ref` sibling, annotations included
    #[test_case("http://json-schema.org/draft-04/schema#"; "draft 4")]
    #[test_case("http://json-schema.org/draft-06/schema#"; "draft 6")]
    #[test_case("http://json-schema.org/draft-07/schema#"; "draft 7")]
    fn ref_siblings_produce_no_annotations(dialect: &str) {
        let schema = json!({
            "$schema": dialect,
            "definitions": {"a": {"type": "string"}},
            "$ref": "#/definitions/a",
            "type": "integer",
            "title": "T",
            "x-unknown": 1
        });
        let root = node("", "");
        let reference = node("/$ref", "/definitions/a");
        let target_type = node("/$ref/type", "/definitions/a/type");
        assert_eq!(
            outputs(&schema),
            (
                json!({"valid": true, "details": [root, reference, target_type]}),
                with_details(root, vec![with_details(reference, vec![target_type])]),
                0
            )
        );
    }

    #[test]
    fn draft7_unknown_keywords_without_ref_are_annotations() {
        let schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "definitions": {"a": {"type": "string"}},
            "type": "string",
            "title": "T",
            "x-unknown": 1
        });
        let mut root = node("", "");
        root["annotations"] = json!({"title": "T", "x-unknown": 1});
        let target_type = node("/type", "/type");
        assert_eq!(
            outputs(&schema),
            (
                json!({"valid": true, "details": [root, target_type]}),
                with_details(root, vec![target_type]),
                1
            )
        );
    }

    #[test]
    fn draft2020_ref_siblings_apply() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": {"a": {"type": "string"}},
            "$ref": "#/$defs/a",
            "maxLength": 5,
            "title": "T",
            "x-unknown": 1
        });
        let mut root = node("", "");
        root["annotations"] = json!({"title": "T", "x-unknown": 1});
        let max_length = node("/maxLength", "/maxLength");
        let reference = node("/$ref", "/$defs/a");
        let target_type = node("/$ref/type", "/$defs/a/type");
        assert_eq!(
            outputs(&schema),
            (
                json!({"valid": true, "details": [root, max_length, reference, target_type]}),
                with_details(
                    root,
                    vec![max_length, with_details(reference, vec![target_type])]
                ),
                1
            )
        );
    }
}
