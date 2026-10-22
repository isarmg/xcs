#[cfg(test)]
mod tests {
    use crate::{Draft, Registry};
    use json_codec::json;

    #[test]
    fn test_lookup_trivial_dynamic_ref() {
        let one = Draft::Draft202012.create_resource(json!({"$dynamicAnchor": "foo"}));
        let registry = Registry::new()
            .add("http://example.com", &one)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));
        let resolved = resolver.lookup("#foo").expect("Lookup failed");
        assert_eq!(resolved.contents(), one.contents());
    }

    #[test]
    fn test_multiple_lookup_trivial_dynamic_ref() {
        let true_resource = Draft::Draft202012.create_resource(json!(true));
        let root = Draft::Draft202012.create_resource(json!({
            "$id": "http://example.com",
            "$dynamicAnchor": "fooAnchor",
            "$defs": {
                "foo": {
                    "$id": "foo",
                    "$dynamicAnchor": "fooAnchor",
                    "$defs": {
                        "bar": true,
                        "baz": {
                            "$dynamicAnchor": "fooAnchor",
                        },
                    },
                },
            },
        }));

        let registry = Registry::new()
            .extend([
                ("http://example.com", &root),
                ("http://example.com/foo/", &true_resource),
                ("http://example.com/foo/bar", &root),
            ])
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));

        let first = resolver.lookup("").expect("Lookup failed");
        let second = first.resolver().lookup("foo/").expect("Lookup failed");
        let third = second.resolver().lookup("bar").expect("Lookup failed");
        let fourth = third
            .resolver()
            .lookup("#fooAnchor")
            .expect("Lookup failed");
        assert_eq!(fourth.contents(), root.contents());
        assert_eq!(format!("{:?}", fourth.resolver()), "Resolver { base_uri: \"http://example.com\", scopes: \"[http://example.com/foo/, http://example.com, http://example.com]\" }");
    }

    #[test]
    fn dynamic_anchor_in_another_document_uses_that_document_as_base() {
        let root = Draft::Draft202012.create_resource(json!({
            "$id": "https://example.com/root.json",
            "$defs": {"node": {"$dynamicAnchor": "node"}}
        }));
        let tree = Draft::Draft202012.create_resource(json!({
            "$id": "https://example.com/tree.json",
            "$dynamicAnchor": "node"
        }));
        let registry = Registry::new()
            .extend([
                ("https://example.com/root.json", &root),
                ("https://example.com/tree.json", &tree),
            ])
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry.resolver(
            crate::uri::from_str("https://example.com/root.json").expect("Invalid base URI"),
        );

        let tree = resolver.lookup("tree.json").expect("Lookup failed");
        let node = tree.resolver().lookup("#node").expect("Lookup failed");

        assert_eq!(node.contents(), &root.contents()["$defs"]["node"]);
        assert_eq!(
            node.resolver().base_uri().as_str(),
            "https://example.com/root.json"
        );
    }

    #[test]
    fn test_multiple_lookup_dynamic_ref_to_nondynamic_ref() {
        let one = Draft::Draft202012.create_resource(json!({"$anchor": "fooAnchor"}));
        let two = Draft::Draft202012.create_resource(json!({
            "$id": "http://example.com",
            "$dynamicAnchor": "fooAnchor",
            "$defs": {
                "foo": {
                    "$id": "foo",
                    "$dynamicAnchor": "fooAnchor",
                    "$defs": {
                        "bar": true,
                        "baz": {
                            "$dynamicAnchor": "fooAnchor",
                        },
                    },
                },
            },
        }));

        let registry = Registry::new()
            .extend([
                ("http://example.com", &two),
                ("http://example.com/foo/", &one),
                ("http://example.com/foo/bar", &two),
            ])
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));

        let first = resolver.lookup("").expect("Lookup failed");
        let second = first.resolver().lookup("foo/").expect("Lookup failed");
        let third = second.resolver().lookup("bar").expect("Lookup failed");
        let fourth = third
            .resolver()
            .lookup("#fooAnchor")
            .expect("Lookup failed");
        assert_eq!(fourth.contents(), two.contents());
    }

    #[test]
    fn test_unknown_anchor() {
        let schema = Draft::Draft202012.create_resource(json!({
            "$defs": {
                "foo": { "$anchor": "knownAnchor" }
            }
        }));
        let registry = Registry::new()
            .add("http://example.com", schema)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));

        let result = resolver.lookup("#unknownAnchor");
        assert_eq!(
            result.expect_err("Should fail").to_string(),
            "Anchor 'unknownAnchor' does not exist"
        );
    }

    #[test]
    fn test_invalid_anchor_with_slash() {
        let schema = Draft::Draft202012.create_resource(json!({
            "$defs": {
                "foo": { "$anchor": "knownAnchor" }
            }
        }));
        let registry = Registry::new()
            .add("http://example.com", schema)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));

        let result = resolver.lookup("#invalid/anchor");
        assert_eq!(
            result.expect_err("Should fail").to_string(),
            "Anchor 'invalid/anchor' is invalid"
        );
    }

    #[test]
    fn test_lookup_trivial_recursive_ref() {
        let resource = Draft::Draft201909.create_resource(json!({"$recursiveAnchor": true}));
        let registry = Registry::new()
            .add("http://example.com", &resource)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));
        let first = resolver.lookup("").expect("Lookup failed");
        let resolved = first
            .resolver()
            .lookup_recursive_ref()
            .expect("Lookup failed");
        assert_eq!(resolved.contents(), resource.contents());
    }

    #[test]
    fn test_lookup_recursive_ref_to_bool() {
        let true_resource = Draft::Draft201909.create_resource(json!(true));
        let registry = Registry::new()
            .add("http://example.com", &true_resource)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));
        let resolved = resolver.lookup_recursive_ref().expect("Lookup failed");
        assert_eq!(resolved.contents(), true_resource.contents());
    }

    #[test]
    fn test_multiple_lookup_recursive_ref_to_bool() {
        let true_resource = Draft::Draft201909.create_resource(json!(true));
        let root = Draft::Draft201909.create_resource(json!({
            "$id": "http://example.com",
            "$recursiveAnchor": true,
            "$defs": {
                "foo": {
                    "$id": "foo",
                    "$recursiveAnchor": true,
                    "$defs": {
                        "bar": true,
                        "baz": {
                            "$recursiveAnchor": true,
                            "$anchor": "fooAnchor",
                        },
                    },
                },
            },
        }));

        let registry = Registry::new()
            .extend([
                ("http://example.com", &root),
                ("http://example.com/foo/", &true_resource),
                ("http://example.com/foo/bar", &root),
            ])
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));
        let first = resolver.lookup("").expect("Lookup failed");
        let second = first.resolver().lookup("foo/").expect("Lookup failed");
        let third = second.resolver().lookup("bar").expect("Lookup failed");
        let fourth = third
            .resolver()
            .lookup_recursive_ref()
            .expect("Lookup failed");
        assert_eq!(fourth.contents(), root.contents());
    }

    #[test]
    fn test_multiple_lookup_recursive_ref_with_nonrecursive_ref() {
        let one = Draft::Draft201909.create_resource(json!({"$recursiveAnchor": true}));
        let two = Draft::Draft201909.create_resource(json!({
            "$id": "http://example.com",
            "$recursiveAnchor": true,
            "$defs": {
                "foo": {
                    "$id": "foo",
                    "$recursiveAnchor": true,
                    "$defs": {
                        "bar": true,
                        "baz": {
                            "$recursiveAnchor": true,
                            "$anchor": "fooAnchor",
                        },
                    },
                },
            },
        }));
        let three = Draft::Draft201909.create_resource(json!({"$recursiveAnchor": false}));

        let registry = Registry::new()
            .extend([
                ("http://example.com", &three),
                ("http://example.com/foo/", &two),
                ("http://example.com/foo/bar", &one),
            ])
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        let resolver = registry
            .resolver(crate::uri::from_str("http://example.com").expect("Invalid base URI"));
        let first = resolver.lookup("").expect("Lookup failed");
        let second = first.resolver().lookup("foo/").expect("Lookup failed");
        let third = second.resolver().lookup("bar").expect("Lookup failed");
        let fourth = third
            .resolver()
            .lookup_recursive_ref()
            .expect("Lookup failed");
        assert_eq!(fourth.contents(), two.contents());
    }
}
