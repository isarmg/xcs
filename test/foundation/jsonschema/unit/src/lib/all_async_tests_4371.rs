#[cfg(all(test, feature = "resolve-async", not(target_family = "wasm")))]
mod async_tests {
    use std::{collections::HashMap, sync::Arc};

    use serde_json::json;

    use crate::{AsyncRetrieve, Draft, Uri};

    /// Mock async retriever for testing
    #[derive(Clone)]
    struct TestRetriever {
        schemas: HashMap<String, serde_json::Value>,
    }

    impl TestRetriever {
        fn new() -> Self {
            let mut schemas = HashMap::new();
            schemas.insert(
                "https://example.com/user.json".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "integer", "minimum": 0}
                    },
                    "required": ["name"]
                }),
            );
            Self { schemas }
        }
    }

    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    impl AsyncRetrieve for TestRetriever {
        async fn retrieve(
            &self,
            uri: &Uri<String>,
        ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
            self.schemas
                .get(uri.as_str())
                .cloned()
                .ok_or_else(|| "Schema not found".into())
        }
    }

    #[tokio::test]
    async fn test_async_validator_for() {
        let schema = json!({
            "$ref": "https://example.com/user.json"
        });

        let validator = crate::async_options()
            .with_retriever(TestRetriever::new())
            .build(&schema)
            .await
            .unwrap();

        // Valid instance
        assert!(validator.is_valid(&json!({
            "name": "John Doe",
            "age": 30
        })));

        // Invalid instances
        assert!(!validator.is_valid(&json!({
            "age": -5
        })));
        assert!(!validator.is_valid(&json!({
            "name": 123,
            "age": 30
        })));
    }

    #[tokio::test]
    async fn test_async_options_with_draft() {
        let schema = json!({
            "$ref": "https://example.com/user.json"
        });

        let validator = crate::async_options()
            .with_draft(Draft::Draft202012)
            .with_retriever(TestRetriever::new())
            .build(&schema)
            .await
            .unwrap();

        assert!(validator.is_valid(&json!({
            "name": "John Doe",
            "age": 30
        })));
    }

    #[tokio::test]
    async fn test_async_retrieval_failure() {
        let schema = json!({
            "$ref": "https://example.com/nonexistent.json"
        });

        let result = crate::async_options()
            .with_retriever(TestRetriever::new())
            .build(&schema)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Schema not found"));
    }

    #[tokio::test]
    async fn test_async_nested_references() {
        let mut retriever = TestRetriever::new();
        retriever.schemas.insert(
            "https://example.com/nested.json".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "user": { "$ref": "https://example.com/user.json" }
                }
            }),
        );

        let schema = json!({
            "$ref": "https://example.com/nested.json"
        });

        let validator = crate::async_options()
            .with_retriever(retriever)
            .build(&schema)
            .await
            .unwrap();

        // Valid nested structure
        assert!(validator.is_valid(&json!({
            "user": {
                "name": "John Doe",
                "age": 30
            }
        })));

        // Invalid nested structure
        assert!(!validator.is_valid(&json!({
            "user": {
                "age": -5
            }
        })));
    }

    #[tokio::test]
    async fn test_async_with_registry_uses_async_retriever_for_inline_only_refs() {
        let registry = crate::Registry::new().prepare().unwrap();
        let schema = json!({
            "$ref": "https://example.com/user.json"
        });

        let validator = crate::async_options()
            .with_registry(&registry)
            .with_retriever(TestRetriever::new())
            .build(&schema)
            .await
            .unwrap();

        assert!(validator.is_valid(&json!({
            "name": "John Doe",
            "age": 30
        })));
        assert!(!validator.is_valid(&json!({
            "age": -5
        })));
    }

    #[tokio::test]
    async fn test_async_validator_for_basic() {
        let schema = json!({"type": "integer"});

        let validator = crate::async_validator_for(&schema).await.unwrap();

        assert!(validator.is_valid(&json!(42)));
        assert!(!validator.is_valid(&json!("abc")));
    }

    #[tokio::test]
    async fn test_async_build_future_is_send() {
        let schema = Arc::new(json!({
            "$ref": "https://example.com/user.json"
        }));
        let retriever = TestRetriever::new();

        let handle = tokio::spawn({
            let schema = Arc::clone(&schema);
            let retriever = retriever.clone();
            async move {
                crate::async_options()
                    .with_retriever(retriever)
                    .build(&schema)
                    .await
            }
        });

        let validator = handle.await.unwrap().unwrap();
        assert!(validator.is_valid(&json!({
            "name": "John Doe",
            "age": 30
        })));
    }
}
