#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use test_case::test_case;

    #[test]
    fn test_location_default() {
        let loc = Location::default();
        assert_eq!(loc.as_str(), "");
    }

    #[test]
    fn test_location_new() {
        let loc = Location::new();
        assert_eq!(loc.as_str(), "");
    }

    #[test]
    fn test_location_is_empty() {
        let root = Location::new();
        assert!(root.is_empty());
        assert!(!root.join("foo").is_empty());
        assert!(!root.join(0).is_empty());
    }

    #[test]
    fn test_location_join_property() {
        let loc = Location::new();
        let loc = loc.join("property");
        assert_eq!(loc.as_str(), "/property");
    }

    #[test]
    fn test_location_join_index() {
        let loc = Location::new();
        let loc = loc.join(0);
        assert_eq!(loc.as_str(), "/0");
    }

    #[test_case(0, "/0"; "cached index 0")]
    #[test_case(15, "/15"; "cached index 15")]
    #[test_case(16, "/16"; "uncached index 16")]
    #[test_case(100, "/100"; "uncached index 100")]
    fn test_lazy_location_single_index(idx: usize, expected: &str) {
        let root = LazyLocation::new();
        let loc = root.push(idx);
        let location: Location = (&loc).into();
        assert_eq!(location.as_str(), expected);
    }

    #[test]
    fn test_location_join_multiple() {
        let loc = Location::new();
        let loc = loc.join("property").join(0);
        assert_eq!(loc.as_str(), "/property/0");
    }

    #[test]
    fn test_as_bytes() {
        let loc = Location::new().join("test");
        assert_eq!(loc.as_bytes(), b"/test");
    }

    #[test]
    fn test_display_trait() {
        let loc = Location::new().join("property");
        assert_eq!(format!("{loc}"), "/property");
    }

    #[test_case("tilde~character", "/tilde~0character"; "escapes tilde")]
    #[test_case("slash/character", "/slash~1character"; "escapes slash")]
    #[test_case("combo~and/slash", "/combo~0and~1slash"; "escapes tilde and slash combined")]
    #[test_case("multiple~/escapes~", "/multiple~0~1escapes~0"; "multiple escapes")]
    #[test_case("first/segment", "/first~1segment"; "escapes slash in nested segment")]
    fn test_location_escaping(segment: &str, expected: &str) {
        let loc = Location::new().join(segment);
        assert_eq!(loc.as_str(), expected);
    }

    #[test_case("/a/b/c", &[LocationSegment::from("a"), LocationSegment::from("b"), LocationSegment::from("c")]; "location with properties")]
    #[test_case("/1/2/3", &[LocationSegment::Index(1), LocationSegment::Index(2), LocationSegment::Index(3)]; "location with indices")]
    #[test_case("/a/1/b/2", &[
        LocationSegment::from("a"),
        LocationSegment::Index(1),
        LocationSegment::from("b"),
        LocationSegment::Index(2)
    ]; "mixed properties and indices")]
    fn test_into_iter(location: &str, expected_segments: &[LocationSegment]) {
        let loc = Location(Arc::from(location.to_string()));
        assert_eq!(loc.into_iter().collect::<Vec<_>>(), expected_segments);
    }

    #[test_case(vec![LocationSegment::from("a"), LocationSegment::from("b")], "/a/b"; "properties only")]
    #[test_case(vec![LocationSegment::Index(1), LocationSegment::Index(2)], "/1/2"; "indices only")]
    #[test_case(vec![LocationSegment::from("a"), LocationSegment::Index(1)], "/a/1"; "mixed segments")]
    fn test_from_iter(segments: Vec<LocationSegment>, expected: &str) {
        assert_eq!(Location::from_iter(segments).as_str(), expected);
    }

    #[test]
    fn test_roundtrip_join_iter_rebuild_equals() {
        let loc = Location::new().join("a/b").join(2).join("x~y");

        let segments: Vec<_> = loc.into_iter().collect();

        let rebuilt = segments
            .into_iter()
            .fold(Location::new(), |acc, seg| match seg {
                LocationSegment::Property(p) => acc.join(p),
                LocationSegment::Index(i) => acc.join(i),
            });

        assert_eq!(loc, rebuilt);
    }

    #[test]
    fn test_validate_error_instance_path_traverses_instance() {
        let schema = json!({
            "type": "object",
            "properties": {
                "table-node": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": { "~": { "type": "string", "minLength": 1 } },
                        "required": ["~"],
                    }
                }
            },
            "$schema": "https://json-schema.org/draft/2020-12/schema",
        });
        let instance = json!({
            "table-node": [
                { "~": "" },
                { "other-value": "" },
            ],
        });

        let error = crate::validate(&schema, &instance).expect_err("Should fail");

        // Traverse instance using the `instance_path`` segments
        let mut current = &instance;
        for segment in error.instance_path() {
            match segment {
                LocationSegment::Property(property) => {
                    current = &current[property.as_ref()];
                }
                LocationSegment::Index(idx) => {
                    current = &current[idx];
                }
            }
        }
        assert_eq!(
            current,
            instance
                .pointer("/table-node/0/~0")
                .expect("Pointer is valid")
        );
    }

    #[test]
    fn test_deep_path_heap_allocation() {
        // Create a schema with >16 levels of nesting to exercise heap allocation path
        const DEPTH: usize = 20;

        let mut schema = json!({"type": "integer"});
        for i in (0..DEPTH).rev() {
            schema = json!({
                "type": "object",
                "properties": {
                    format!("level{i}"): schema
                }
            });
        }

        let mut instance = json!("not an integer");
        for i in (0..DEPTH).rev() {
            instance = json!({
                format!("level{i}"): instance
            });
        }

        let error = crate::validate(&schema, &instance).expect_err("Should fail");

        // Verify instance_path has correct depth (>16 exercises heap allocation)
        let instance_path = error.instance_path();
        assert_eq!(instance_path.into_iter().count(), DEPTH);
        let expected_instance_path = (0..DEPTH).fold(String::new(), |mut acc, i| {
            use std::fmt::Write;
            write!(acc, "/level{i}").unwrap();
            acc
        });
        assert_eq!(instance_path.as_str(), expected_instance_path);

        // Verify schema_path also has correct depth
        let schema_path = error.schema_path();
        let expected_schema_path = (0..DEPTH).fold(String::new(), |mut acc, i| {
            use std::fmt::Write;
            write!(acc, "/properties/level{i}").unwrap();
            acc
        }) + "/type";
        assert_eq!(schema_path.as_str(), expected_schema_path);
    }

    #[test]
    fn test_dynamic_ref_evaluation_path() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$dynamicAnchor": "node",
            "type": "object",
            "properties": {
                "data": {"type": "string"},
                "child": {"$dynamicRef": "#node"}
            }
        });
        let instance = json!({
            "data": "parent",
            "child": {
                "data": 123
            }
        });

        let error = crate::validate(&schema, &instance).expect_err("Should fail");

        // schema_path is canonical (anchor resolves to root)
        assert_eq!(error.schema_path().as_str(), "/properties/data/type");
        // evaluation_path includes $dynamicRef traversal
        assert_eq!(
            error.evaluation_path().as_str(),
            "/properties/child/$dynamicRef/properties/data/type"
        );
        assert_eq!(error.instance_path().as_str(), "/child/data");
    }

    #[test]
    fn test_nested_ref_evaluation_path() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "properties": {
                "order": { "$ref": "#/$defs/Order" }
            },
            "$defs": {
                "Order": {
                    "properties": {
                        "item": { "$ref": "#/$defs/Item" }
                    }
                },
                "Item": { "type": "string" }
            }
        });
        let instance = json!({ "order": { "item": 123 } });

        let error = crate::validate(&schema, &instance).expect_err("Should fail");

        assert_eq!(error.schema_path().as_str(), "/$defs/Item/type");
        assert_eq!(
            error.evaluation_path().as_str(),
            "/properties/order/$ref/properties/item/$ref/type"
        );
        assert_eq!(error.instance_path().as_str(), "/order/item");
    }

    #[test]
    fn test_ref_to_boolean_schema_evaluation_path() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": {
                "deny": false
            },
            "$ref": "#/$defs/deny"
        });

        let instance = json!("anything");
        let error = crate::validate(&schema, &instance).expect_err("Should fail");

        assert_eq!(error.schema_path().as_str(), "/$defs/deny");
        assert_eq!(error.evaluation_path().as_str(), "/$ref");
    }

    #[test]
    fn test_no_ref_evaluation_path_equals_schema_path() {
        let schema = json!({
            "properties": {
                "name": { "type": "string" }
            }
        });
        let instance = json!({ "name": 123 });

        let error = crate::validate(&schema, &instance).expect_err("Should fail");

        assert_eq!(error.schema_path().as_str(), "/properties/name/type");
        assert_eq!(
            error.evaluation_path().as_str(),
            error.schema_path().as_str()
        );
    }
}
