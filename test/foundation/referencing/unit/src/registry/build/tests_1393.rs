#[cfg(test)]
mod tests {
    use std::{error::Error as _, sync::Arc};

    use ahash::AHashMap;
    use fluent_uri::Uri;
    use json_codec::{json, Value};
    use test_case::test_case;

    use crate::{
        registry::SPECIFICATIONS, uri::from_str, Anchor, Draft, Registry, Resource, Retrieve,
    };

    #[test]
    fn test_invalid_uri_on_registry_creation() {
        let schema = Draft::Draft202012.create_resource(json!({}));
        let result = Registry::new().add(":/example.com", schema);
        let error = result.expect_err("Should fail");

        assert_eq!(
            error.to_string(),
            "Invalid URI reference ':/example.com': unexpected character at index 0"
        );
        let source_error = error.source().expect("Should have a source");
        let inner_source = source_error.source().expect("Should have a source");
        assert_eq!(inner_source.to_string(), "unexpected character at index 0");
    }

    #[test]
    fn test_lookup_unresolvable_url() {
        // Create a registry with a single resource
        let schema = Draft::Draft202012.create_resource(json!({
            "type": "object",
            "properties": {
                "foo": { "type": "string" }
            }
        }));
        let registry = Registry::new()
            .add("http://example.com/schema1", schema)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");

        // Attempt to create a resolver for a URL not in the registry
        let resolver = registry.resolver(
            from_str("http://example.com/non_existent_schema").expect("Invalid base URI"),
        );

        let result = resolver.lookup("");

        assert_eq!(
            result.unwrap_err().to_string(),
            "Resource 'http://example.com/non_existent_schema' is not present in a registry and retrieving it failed: Retrieving external resources is not supported once the registry is populated"
        );
    }

    // Drafts 4-7 allow a fragment in `$id`, which makes the resource's base URI carry one.
    // RFC 3986 5.2.1 leaves the base URI's fragment undefined, so it must be dropped when
    // resolving a reference against it rather than rejected. See GH-1473.
    #[test]
    fn test_lookup_relative_ref_against_base_with_fragment() {
        let root = json!({
            "$schema": "http://json-schema.org/draft-07/schema",
            "$id": "release-it#release-it",
            "properties": {"git": {"$ref": "./git.json"}}
        });
        let registry = Registry::new()
            .retriever(create_test_retriever(&[(
                "https://example.com/schema/git.json",
                json!({"$schema": "http://json-schema.org/draft-07/schema", "type": "object"}),
            )]))
            .add("https://example.com/schema/release-it.json", root)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");

        let resolver = registry.resolver(
            from_str("https://example.com/schema/release-it#release-it").expect("Invalid base URI"),
        );

        let resolved = resolver.lookup("./git.json").expect("Should resolve");
        assert_eq!(
            resolved.contents(),
            &json!({"$schema": "http://json-schema.org/draft-07/schema", "type": "object"})
        );
    }

    #[test]
    fn test_registry_can_be_built_from_borrowed_resources() {
        let schema = json!({"type": "string"});
        let registry = Registry::new()
            .add("urn:root", &schema)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");
        assert!(registry.contains_resource("urn:root"));
    }

    // Move each level in; `json!` interpolation re-serializes recursively and would overflow
    // during construction instead of in the code under test.
    fn nested_all_of(depth: usize) -> Value {
        let mut schema = json!({"type": "integer"});
        for _ in 0..depth {
            let mut map = json_codec::Map::new();
            map.insert("allOf".to_string(), Value::Array(vec![schema]));
            schema = Value::Object(map);
        }
        schema
    }

    // Preparing a deeply nested resource must not recurse per level. The value is leaked, not
    // dropped: `serde_json` drops `Value` recursively and would overflow the test-thread stack.
    // Ignored under Miri, which does not model real thread stacks and flags the deliberate leak.
    #[cfg_attr(miri, ignore)]
    #[test]
    fn deeply_nested_owned_resource_does_not_overflow() {
        let registry = Registry::new()
            .add("https://example.com/deep", nested_all_of(5000))
            .expect("valid resource")
            .prepare()
            .expect("prepare should not overflow");
        assert!(registry.contains_resource("https://example.com/deep"));
        std::mem::forget(registry);
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn deeply_nested_borrowed_resource_does_not_overflow() {
        let schema = nested_all_of(5000);
        let registry = Registry::new()
            .add("https://example.com/deep", &schema)
            .expect("valid resource")
            .prepare()
            .expect("prepare should not overflow");
        assert!(registry.contains_resource("https://example.com/deep"));
        drop(registry);
        std::mem::forget(schema);
    }

    #[test]
    fn test_prepare_builds_local_entries_for_borrowed_and_owned() {
        let root = json!({"$ref": "http://example.com/remote"});
        let remote = json!({"type": "string"});
        let registry = Registry::new()
            .retriever(create_test_retriever(&[(
                "http://example.com/remote",
                remote.clone(),
            )]))
            .add("http://example.com/root", &root)
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");

        let root_uri = from_str("http://example.com/root").expect("Invalid root URI");
        let remote_uri = from_str("http://example.com/remote").expect("Invalid remote URI");

        let root_resource = registry
            .resource_by_uri(&root_uri)
            .expect("Borrowed root should be available from prepared local entries");
        let remote_resource = registry
            .resource_by_uri(&remote_uri)
            .expect("Owned retrieved document should be available from prepared local entries");

        assert_eq!(root_resource.contents(), &root);
        assert_eq!(remote_resource.contents(), &remote);
    }

    #[test]
    fn test_prepare_collects_relative_refs_for_each_borrowed_alias() {
        let schema = json!({"$ref": "remote.json"});
        let registry = Registry::new()
            .retriever(create_test_retriever(&[
                ("http://a/remote.json", json!({"type": "string"})),
                ("http://b/remote.json", json!({"type": "integer"})),
            ]))
            .add("http://a/root.json", &schema)
            .expect("First alias should be accepted")
            .add("http://b/root.json", &schema)
            .expect("Second alias should be accepted")
            .prepare()
            .expect("Registry should prepare");

        assert!(registry.contains_resource("http://a/remote.json"));
        assert!(registry.contains_resource("http://b/remote.json"));

        let resolver = registry.resolver(from_str("http://b/root.json").expect("Invalid root URI"));
        assert_eq!(
            resolver
                .lookup("remote.json")
                .expect("Borrowed alias should resolve its own relative ref")
                .contents(),
            &json!({"type": "integer"})
        );
    }

    #[test]
    fn test_prepare_collects_deferred_relative_refs_for_each_borrowed_alias() {
        let schema = json!({
            "$defs": {
                "remote": { "$ref": "remote.json" }
            },
            "$ref": "#/$defs/remote"
        });
        let registry = Registry::new()
            .retriever(create_test_retriever(&[
                ("http://a/remote.json", json!({"type": "string"})),
                ("http://b/remote.json", json!({"type": "integer"})),
            ]))
            .add("http://a/root.json", &schema)
            .expect("First alias should be accepted")
            .add("http://b/root.json", &schema)
            .expect("Second alias should be accepted")
            .prepare()
            .expect("Registry should prepare");

        assert!(registry.contains_resource("http://a/remote.json"));
        assert!(registry.contains_resource("http://b/remote.json"));
    }

    #[test]
    fn test_prepare_collects_deferred_refs_using_inherited_base_uri() {
        let schema = json!({
            "$id": "http://localhost:1234/scope_change_defs2.json",
            "type": "object",
            "properties": {
                "list": {"$ref": "#/definitions/baz/definitions/bar"}
            },
            "definitions": {
                "baz": {
                    "$id": "baseUriChangeFolderInSubschema/",
                    "definitions": {
                        "bar": {
                            "type": "array",
                            "items": {"$ref": "folderInteger.json"}
                        }
                    }
                }
            }
        });

        let registry = Registry::new()
            .retriever(create_test_retriever(&[(
                "http://localhost:1234/baseUriChangeFolderInSubschema/folderInteger.json",
                json!({"type": "integer"}),
            )]))
            .add("http://localhost:1234/scope_change_defs2.json", &schema)
            .expect("Schema should be accepted")
            .prepare()
            .expect("Registry should prepare");

        assert!(registry.contains_resource(
            "http://localhost:1234/baseUriChangeFolderInSubschema/folderInteger.json"
        ));
        assert!(!registry.contains_resource("http://localhost:1234/folderInteger.json"));
    }

    #[test]
    fn test_prepare_populates_local_entries_for_subresources_and_anchors() {
        let registry = Registry::new()
            .add(
                "http://example.com/root",
                json!({
                    "$defs": {
                        "embedded": {
                            "$id": "http://example.com/embedded",
                            "$anchor": "node",
                            "type": "string"
                        }
                    }
                }),
            )
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");

        let embedded_uri = from_str("http://example.com/embedded").expect("Invalid embedded URI");
        let embedded_resource = registry
            .resource_by_uri(&embedded_uri)
            .expect("Embedded subresource should be available from prepared local entries");
        assert_eq!(
            embedded_resource.contents(),
            &json!({
                "$id": "http://example.com/embedded",
                "$anchor": "node",
                "type": "string"
            })
        );

        let embedded_anchor = registry
            .anchor(&embedded_uri, "node")
            .expect("Embedded anchor should be available from prepared local entries");
        match embedded_anchor {
            Anchor::Default { resource, .. } => assert_eq!(
                resource.contents(),
                &json!({
                    "$id": "http://example.com/embedded",
                    "$anchor": "node",
                    "type": "string"
                })
            ),
            Anchor::Dynamic { .. } => panic!("Expected a default anchor"),
        }
    }

    #[test]
    fn test_prepare_merges_anchor_entries_for_shared_effective_uri() {
        let registry = Registry::new()
            .add(
                "http://example.com/root",
                json!({
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                    "$defs": {
                        "first": {
                            "$anchor": "first",
                            "type": "string"
                        },
                        "second": {
                            "$anchor": "second",
                            "type": "integer"
                        }
                    }
                }),
            )
            .expect("Invalid resources")
            .prepare()
            .expect("Invalid resources");

        let resolver = registry.resolver(from_str("http://example.com/root").expect("Invalid URI"));

        assert_eq!(
            resolver
                .lookup("#first")
                .expect("First anchor should resolve")
                .contents(),
            &json!({
                "$anchor": "first",
                "type": "string"
            })
        );
        assert_eq!(
            resolver
                .lookup("#second")
                .expect("Second anchor should resolve")
                .contents(),
            &json!({
                "$anchor": "second",
                "type": "integer"
            })
        );
    }

    #[test]
    fn test_relative_uri_without_base() {
        let schema = Draft::Draft202012.create_resource(json!({"$ref": "./virtualNetwork.json"}));
        let error = Registry::new()
            .add("json-schema:///", schema)
            .expect("Root resource should be accepted")
            .prepare()
            .expect_err("Should fail");
        assert_eq!(error.to_string(), "Resource './virtualNetwork.json' is not present in a registry and retrieving it failed: No base URI is available");
    }

    #[test]
    fn test_prepare_requires_registered_custom_meta_schema() {
        let base_registry = Registry::new()
            .add(
                "http://example.com/root",
                Resource::from_contents(json!({"type": "object"})),
            )
            .expect("Base registry should be created")
            .prepare()
            .expect("Base registry should be created");

        let custom_schema = Resource::from_contents(json!({
            "$id": "http://example.com/custom",
            "$schema": "http://example.com/meta/custom",
            "type": "string"
        }));

        let error = base_registry
            .add("http://example.com/custom", custom_schema)
            .expect("Schema should be accepted")
            .prepare()
            .expect_err("Extending registry must fail when the custom $schema is not registered");

        let error_msg = error.to_string();
        assert_eq!(
            error_msg,
            "Unknown meta-schema: 'http://example.com/meta/custom'. Custom meta-schemas must be registered in the registry before use"
        );
    }

    #[test_case("https://json-schema.org/draft/2020-12/meta/format-assertion")]
    #[test_case("https://json-schema.org/draft/2019-09/meta/format")]
    fn test_prepare_accepts_bundled_meta_schema(meta_schema: &str) {
        let registry = Registry::new()
            .add(
                "http://example.com/root",
                Resource::from_contents(json!({"$schema": meta_schema, "format": "ipv4"})),
            )
            .expect("Schema should be accepted")
            .prepare()
            .expect("Bundled meta-schemas need no registration");

        let meta_uri = from_str(meta_schema).expect("Valid URI");
        assert!(registry.resource_by_uri(&meta_uri).is_some());
    }

    #[test]
    fn test_prepare_accepts_registered_custom_meta_schema_fragment() {
        let meta_schema = Resource::from_contents(json!({
            "$id": "http://example.com/meta/custom#",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object"
        }));

        let registry = Registry::new()
            .add("http://example.com/meta/custom#", meta_schema)
            .expect("Meta-schema should be registered successfully")
            .prepare()
            .expect("Meta-schema should be registered successfully");

        let schema = Resource::from_contents(json!({
            "$id": "http://example.com/schemas/my-schema",
            "$schema": "http://example.com/meta/custom#",
            "type": "string"
        }));

        registry
            .add("http://example.com/schemas/my-schema", schema)
            .expect("Schema should be accepted")
            .prepare()
            .expect("Schema should accept registered meta-schema URI with trailing '#'");
    }

    #[test]
    fn test_chained_custom_meta_schemas() {
        // Meta-schema B (uses standard Draft 2020-12)
        let meta_schema_b = json!({
            "$id": "json-schema:///meta/level-b",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/validation": true,
            },
            "type": "object",
            "properties": {
                "customProperty": {"type": "string"}
            }
        });

        // Meta-schema A (uses Meta-schema B)
        let meta_schema_a = json!({
            "$id": "json-schema:///meta/level-a",
            "$schema": "json-schema:///meta/level-b",
            "customProperty": "level-a-meta",
            "type": "object"
        });

        // Schema (uses Meta-schema A)
        let schema = json!({
            "$id": "json-schema:///schemas/my-schema",
            "$schema": "json-schema:///meta/level-a",
            "customProperty": "my-schema",
            "type": "string"
        });

        // Register all meta-schemas and schema in a chained manner
        // All resources are provided upfront, so no external retrieval should occur
        Registry::new()
            .add(
                "json-schema:///meta/level-b",
                Resource::from_contents(meta_schema_b),
            )
            .expect("Meta-schema should be accepted")
            .add(
                "json-schema:///meta/level-a",
                Resource::from_contents(meta_schema_a),
            )
            .expect("Meta-schema should be accepted")
            .add(
                "json-schema:///schemas/my-schema",
                Resource::from_contents(schema),
            )
            .expect("Schema should be accepted")
            .prepare()
            .expect("Chained custom meta-schemas should be accepted when all are registered");
    }

    struct TestRetriever {
        schemas: AHashMap<String, Value>,
    }

    impl TestRetriever {
        fn new(schemas: AHashMap<String, Value>) -> Self {
            TestRetriever { schemas }
        }
    }

    impl Retrieve for TestRetriever {
        fn retrieve(
            &self,
            uri: &Uri<String>,
        ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            if let Some(value) = self.schemas.get(uri.as_str()) {
                Ok(value.clone())
            } else {
                Err(format!("Failed to find {uri}").into())
            }
        }
    }

    fn create_test_retriever(schemas: &[(&str, Value)]) -> TestRetriever {
        TestRetriever::new(
            schemas
                .iter()
                .map(|&(k, ref v)| (k.to_string(), v.clone()))
                .collect(),
        )
    }

    #[test]
    fn test_registry_builder_uses_custom_draft() {
        let registry = Registry::new()
            .draft(Draft::Draft4)
            .add("urn:test", json!({}))
            .expect("Resource should be accepted")
            .prepare()
            .expect("Registry should prepare");

        let uri = from_str("urn:test").expect("Invalid test URI");
        assert_eq!(
            registry.resource_by_uri(&uri).unwrap().draft(),
            Draft::Draft4
        );
    }

    #[test]
    fn test_registry_builder_uses_custom_retriever() {
        let registry = Registry::new()
            .retriever(create_test_retriever(&[(
                "http://example.com/remote",
                json!({"type": "string"}),
            )]))
            .add(
                "http://example.com/root",
                json!({"$ref": "http://example.com/remote"}),
            )
            .expect("Resource should be accepted")
            .prepare()
            .expect("Registry should prepare");

        assert!(registry.contains_resource("http://example.com/remote"));
    }

    #[test]
    fn test_default_retriever_with_remote_refs() {
        let result = Registry::new()
            .add(
                "http://example.com/schema1",
                Resource::from_contents(json!({"$ref": "http://example.com/schema2"})),
            )
            .expect("Resource should be accepted")
            .prepare();
        let error = result.expect_err("Should fail");
        assert_eq!(error.to_string(), "Resource 'http://example.com/schema2' is not present in a registry and retrieving it failed: Default retriever does not fetch resources");
        assert!(error.source().is_some());
    }

    #[test]
    fn test_prepared_registry_can_be_extended_via_add() {
        let original = Registry::new()
            .add("urn:one", json!({"type": "string"}))
            .expect("Resource should be accepted")
            .prepare()
            .expect("Registry should prepare");

        let registry = original
            .add("urn:two", json!({"type": "integer"}))
            .expect("Resource should be accepted")
            .prepare()
            .expect("Registry should prepare");

        assert!(original.contains_resource("urn:one"));
        assert!(!original.contains_resource("urn:two"));
        assert!(registry.contains_resource("urn:one"));
        assert!(registry.contains_resource("urn:two"));
    }

    #[test]
    fn test_prepared_registry_can_be_extended_via_extend() {
        let original = Registry::new()
            .add("urn:one", json!({"type": "string"}))
            .expect("Resource should be accepted")
            .prepare()
            .expect("Registry should prepare");

        let registry = original
            .extend([
                ("urn:two", json!({"type": "integer"})),
                ("urn:three", json!({"type": "boolean"})),
            ])
            .expect("Resources should be accepted")
            .prepare()
            .expect("Registry should prepare");

        assert!(original.contains_resource("urn:one"));
        assert!(!original.contains_resource("urn:two"));
        assert!(!original.contains_resource("urn:three"));
        assert!(registry.contains_resource("urn:one"));
        assert!(registry.contains_resource("urn:two"));
        assert!(registry.contains_resource("urn:three"));
    }

    #[test]
    fn test_registry_builder_accepts_borrowed_values() {
        let schema = json!({"type": "string"});
        let registry = Registry::new()
            .add("urn:test", &schema)
            .expect("Resource should be accepted")
            .prepare()
            .expect("Registry should prepare");

        assert!(registry.contains_resource("urn:test"));
    }

    #[test]
    fn test_registry_builder_accepts_arc_values() {
        let schema = Arc::new(json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/root",
            "$defs": { "item": { "type": "string" } },
            "items": { "$ref": "#/$defs/item" }
        }));
        let registry = Registry::new()
            .add("https://example.com/root", Arc::clone(&schema))
            .expect("Resource should be accepted")
            .prepare()
            .expect("Registry should prepare");

        assert!(registry.contains_resource("https://example.com/root"));
        let uri = from_str("https://example.com/root").expect("Invalid test URI");
        let resource = registry.resource_by_uri(&uri).unwrap();
        // Draft detected from the shared contents, which flowed through the crawl.
        assert_eq!(resource.draft(), Draft::Draft202012);
        assert!(resource.contents().get("$defs").is_some());
        // Adding the Arc must not consume or clone the contents; the caller keeps it.
        assert_eq!(schema["$id"], json!("https://example.com/root"));
    }

    #[test]
    fn test_registry_builder_accepts_borrowed_resources() {
        let schema = Draft::Draft4.create_resource(json!({"type": "string"}));
        let registry = Registry::new()
            .add("urn:test", &schema)
            .expect("Resource should be accepted")
            .prepare()
            .expect("Registry should prepare");

        let uri = from_str("urn:test").expect("Invalid test URI");
        assert_eq!(
            registry.resource_by_uri(&uri).unwrap().draft(),
            Draft::Draft4
        );
    }

    #[test]
    fn test_registry_with_duplicate_input_uris() {
        let registry = Registry::new()
            .add(
                "http://example.com/schema",
                json!({
                    "type": "object",
                    "properties": {
                        "foo": { "type": "string" }
                    }
                }),
            )
            .expect("First resource should be accepted")
            .add(
                "http://example.com/schema",
                json!({
                    "type": "object",
                    "properties": {
                        "bar": { "type": "number" }
                    }
                }),
            )
            .expect("Second resource should overwrite the first")
            .prepare()
            .expect("Registry should prepare");

        let uri = from_str("http://example.com/schema").expect("Invalid schema URI");
        let resource = registry.resource_by_uri(&uri).unwrap();
        let properties = resource
            .contents()
            .get("properties")
            .and_then(|v| v.as_object())
            .unwrap();

        assert!(
            !properties.contains_key("foo"),
            "Registry should replace the earlier explicit input resource"
        );
        assert!(properties.contains_key("bar"));
    }

    #[test]
    fn test_resolver_debug() {
        let registry = SPECIFICATIONS
            .add("http://example.com", json!({}))
            .expect("Invalid resource")
            .prepare()
            .expect("Invalid resource");
        let resolver =
            registry.resolver(from_str("http://127.0.0.1/schema").expect("Invalid base URI"));
        assert_eq!(
            format!("{resolver:?}"),
            "Resolver { base_uri: \"http://127.0.0.1/schema\", scopes: \"[]\" }"
        );
    }

    #[test]
    fn test_prepare_with_specifications_registry() {
        let registry = SPECIFICATIONS
            .add("http://example.com", json!({}))
            .expect("Invalid resource")
            .prepare()
            .expect("Invalid resource");
        let resolver = registry.resolver(from_str("").expect("Invalid base URI"));
        let resolved = resolver
            .lookup("http://json-schema.org/draft-06/schema#/definitions/schemaArray")
            .expect("Lookup failed");
        assert_eq!(
            resolved.contents(),
            &json!({
                "type": "array",
                "minItems": 1,
                "items": { "$ref": "#" }
            })
        );
    }

    #[test]
    fn test_prepare_preserves_existing_local_entries() {
        let original = Registry::new()
            .add(
                "http://example.com/root",
                Resource::from_contents(json!({
                    "$defs": {
                        "embedded": {
                            "$id": "http://example.com/embedded",
                            "type": "string"
                        }
                    }
                })),
            )
            .expect("Invalid root schema")
            .prepare()
            .expect("Invalid root schema");

        let extended = original
            .add(
                "http://example.com/other",
                Resource::from_contents(json!({"type": "number"})),
            )
            .expect("Registry extension should succeed")
            .prepare()
            .expect("Registry extension should succeed");

        let resolver = extended.resolver(from_str("").expect("Invalid base URI"));
        let embedded = resolver
            .lookup("http://example.com/embedded")
            .expect("Embedded subresource URI should stay indexed after extension");
        assert_eq!(
            embedded.contents(),
            &json!({
                "$id": "http://example.com/embedded",
                "type": "string"
            })
        );
    }

    #[test]
    fn test_invalid_reference() {
        let resource = Draft::Draft202012.create_resource(json!({"$schema": "$##"}));
        let _ = Registry::new()
            .add("http://#/", resource)
            .and_then(crate::registry::RegistryBuilder::prepare);
    }
}
