#[cfg(all(test, feature = "retrieve-async"))]
mod async_tests {
    use crate::{uri, DefaultRetriever, Draft, Registry, Resource, Uri};
    use ahash::AHashMap;
    use serde_json::{json, Value};
    use std::{
        error::Error,
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct TestAsyncRetriever {
        schemas: AHashMap<String, Value>,
    }

    impl TestAsyncRetriever {
        fn with_schema(uri: impl Into<String>, schema: Value) -> Self {
            TestAsyncRetriever {
                schemas: { AHashMap::from_iter([(uri.into(), schema)]) },
            }
        }
    }

    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    impl crate::AsyncRetrieve for TestAsyncRetriever {
        async fn retrieve(
            &self,
            uri: &Uri<String>,
        ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            self.schemas
                .get(uri.as_str())
                .cloned()
                .ok_or_else(|| "Schema not found".into())
        }
    }

    #[tokio::test]
    async fn test_default_async_retriever_with_remote_refs() {
        let result = Registry::new()
            .async_retriever(DefaultRetriever)
            .add(
                "http://example.com/schema1",
                Resource::from_contents(json!({"$ref": "http://example.com/schema2"})),
            )
            .expect("Resource should be accepted")
            .async_prepare()
            .await;

        let error = result.expect_err("Should fail");
        assert_eq!(error.to_string(), "Resource 'http://example.com/schema2' is not present in a registry and retrieving it failed: Default retriever does not fetch resources");
        assert!(error.source().is_some());
    }

    #[tokio::test]
    async fn test_async_prepare() {
        let _registry = Registry::new()
            .async_retriever(DefaultRetriever)
            .add("", Draft::default().create_resource(json!({})))
            .expect("Invalid resources")
            .async_prepare()
            .await
            .expect("Invalid resources");
    }

    #[tokio::test]
    async fn test_async_registry_with_duplicate_input_uris() {
        let registry = Registry::new()
            .async_retriever(DefaultRetriever)
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
            .async_prepare()
            .await
            .expect("Registry should prepare");

        let uri = uri::from_str("http://example.com/schema").expect("Invalid schema URI");
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

    #[tokio::test]
    async fn test_registry_builder_async_prepare_uses_async_retriever() {
        let registry = Registry::new()
            .async_retriever(TestAsyncRetriever::with_schema(
                "http://example.com/schema2",
                json!({"type": "object"}),
            ))
            .add(
                "http://example.com",
                json!({"$ref": "http://example.com/schema2"}),
            )
            .expect("Resource should be accepted")
            .async_prepare()
            .await
            .expect("Registry should prepare");

        let resolver = registry.resolver(uri::from_str("").expect("Invalid base URI"));
        let resolved = resolver
            .lookup("http://example.com/schema2")
            .expect("Lookup failed");
        assert_eq!(resolved.contents(), &json!({"type": "object"}));
    }

    #[tokio::test]
    async fn test_async_prepare_with_remote_resource() {
        let retriever = TestAsyncRetriever::with_schema(
            "http://example.com/schema2",
            json!({"type": "object"}),
        );

        let registry = Registry::new()
            .async_retriever(retriever)
            .add(
                "http://example.com",
                Resource::from_contents(json!({"$ref": "http://example.com/schema2"})),
            )
            .expect("Invalid resource")
            .async_prepare()
            .await
            .expect("Invalid resource");

        let resolver = registry.resolver(uri::from_str("").expect("Invalid base URI"));
        let resolved = resolver
            .lookup("http://example.com/schema2")
            .expect("Lookup failed");
        assert_eq!(resolved.contents(), &json!({"type": "object"}));
    }

    #[tokio::test]
    async fn test_async_prepare_preserves_existing_local_entries() {
        let original = Registry::new()
            .async_retriever(DefaultRetriever)
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
            .async_prepare()
            .await
            .expect("Invalid root schema");

        let extended = original
            .add(
                "http://example.com/other",
                Resource::from_contents(json!({"type": "number"})),
            )
            .expect("Registry extension should succeed")
            .async_prepare()
            .await
            .expect("Registry extension should succeed");

        let resolver = extended.resolver(uri::from_str("").expect("Invalid base URI"));
        let embedded = resolver
            .lookup("http://example.com/embedded")
            .expect("Embedded subresource URI should stay indexed after async extension");
        assert_eq!(
            embedded.contents(),
            &json!({
                "$id": "http://example.com/embedded",
                "type": "string"
            })
        );
    }

    #[tokio::test]
    async fn test_async_registry_with_multiple_refs() {
        let retriever = TestAsyncRetriever {
            schemas: AHashMap::from_iter([
                (
                    "http://example.com/schema2".to_string(),
                    json!({"type": "object"}),
                ),
                (
                    "http://example.com/schema3".to_string(),
                    json!({"type": "string"}),
                ),
            ]),
        };

        let registry = Registry::new()
            .async_retriever(retriever)
            .add(
                "http://example.com/schema1",
                Resource::from_contents(json!({
                    "type": "object",
                    "properties": {
                        "obj": {"$ref": "http://example.com/schema2"},
                        "str": {"$ref": "http://example.com/schema3"}
                    }
                })),
            )
            .expect("Invalid resource")
            .async_prepare()
            .await
            .expect("Invalid resource");

        let resolver = registry.resolver(uri::from_str("").expect("Invalid base URI"));

        // Check both references are resolved correctly
        let resolved2 = resolver
            .lookup("http://example.com/schema2")
            .expect("Lookup failed");
        assert_eq!(resolved2.contents(), &json!({"type": "object"}));

        let resolved3 = resolver
            .lookup("http://example.com/schema3")
            .expect("Lookup failed");
        assert_eq!(resolved3.contents(), &json!({"type": "string"}));
    }

    #[tokio::test]
    async fn test_async_registry_with_nested_refs() {
        let retriever = TestAsyncRetriever {
            schemas: AHashMap::from_iter([
                (
                    "http://example.com/address".to_string(),
                    json!({
                        "type": "object",
                        "properties": {
                            "street": {"type": "string"},
                            "city": {"$ref": "http://example.com/city"}
                        }
                    }),
                ),
                (
                    "http://example.com/city".to_string(),
                    json!({
                        "type": "string",
                        "minLength": 1
                    }),
                ),
            ]),
        };

        let registry = Registry::new()
            .async_retriever(retriever)
            .add(
                "http://example.com/person",
                Resource::from_contents(json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "address": {"$ref": "http://example.com/address"}
                    }
                })),
            )
            .expect("Invalid resource")
            .async_prepare()
            .await
            .expect("Invalid resource");

        let resolver = registry.resolver(uri::from_str("").expect("Invalid base URI"));

        // Verify nested reference resolution
        let resolved = resolver
            .lookup("http://example.com/city")
            .expect("Lookup failed");
        assert_eq!(
            resolved.contents(),
            &json!({"type": "string", "minLength": 1})
        );
    }

    // Multiple refs to the same external schema with different fragments were fetched multiple times in async mode.
    #[tokio::test]
    async fn test_async_registry_with_duplicate_fragment_refs() {
        static FETCH_COUNT: AtomicUsize = AtomicUsize::new(0);

        struct CountingRetriever {
            inner: TestAsyncRetriever,
        }

        #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
        #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
        impl crate::AsyncRetrieve for CountingRetriever {
            async fn retrieve(
                &self,
                uri: &Uri<String>,
            ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
                FETCH_COUNT.fetch_add(1, Ordering::SeqCst);
                self.inner.retrieve(uri).await
            }
        }

        FETCH_COUNT.store(0, Ordering::SeqCst);

        let retriever = CountingRetriever {
            inner: TestAsyncRetriever::with_schema(
                "http://example.com/external",
                json!({
                    "$defs": {
                        "foo": {
                            "type": "object",
                            "properties": {
                                "nested": { "type": "string" }
                            }
                        },
                        "bar": {
                            "type": "object",
                            "properties": {
                                "value": { "type": "integer" }
                            }
                        }
                    }
                }),
            ),
        };

        // Schema references the same external URL with different fragments
        let registry = Registry::new()
            .async_retriever(retriever)
            .add(
                "http://example.com/main",
                Resource::from_contents(json!({
                    "type": "object",
                    "properties": {
                        "name": { "$ref": "http://example.com/external#/$defs/foo" },
                        "age": { "$ref": "http://example.com/external#/$defs/bar" }
                    }
                })),
            )
            .expect("Invalid resource")
            .async_prepare()
            .await
            .expect("Invalid resource");

        // Should only fetch the external schema once
        let fetches = FETCH_COUNT.load(Ordering::SeqCst);
        assert_eq!(
            fetches, 1,
            "External schema should be fetched only once, but was fetched {fetches} times"
        );

        let resolver =
            registry.resolver(uri::from_str("http://example.com/main").expect("Invalid base URI"));

        // Verify both fragment references resolve correctly
        let foo = resolver
            .lookup("http://example.com/external#/$defs/foo")
            .expect("Lookup failed");
        assert_eq!(
            foo.contents(),
            &json!({
                "type": "object",
                "properties": {
                    "nested": { "type": "string" }
                }
            })
        );

        let bar = resolver
            .lookup("http://example.com/external#/$defs/bar")
            .expect("Lookup failed");
        assert_eq!(
            bar.contents(),
            &json!({
                "type": "object",
                "properties": {
                    "value": { "type": "integer" }
                }
            })
        );
    }
}
