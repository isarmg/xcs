#[cfg(test)]
mod tests {
    #[cfg(all(not(target_arch = "wasm32"), feature = "resolve-file"))]
    use super::path_to_uri;
    #[cfg(all(
        feature = "resolve-http",
        feature = "resolve-async",
        not(target_arch = "wasm32")
    ))]
    use crate::AsyncHttpRetriever;
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    use crate::{HttpOptions, HttpRetriever, HttpRetrieverError};
    use serde_json::json;
    #[cfg(all(not(target_arch = "wasm32"), feature = "resolve-file"))]
    use std::io::Write;

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), feature = "resolve-file"))]
    fn test_retrieve_from_file() {
        let mut temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let external_schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });
        write!(temp_file, "{external_schema}").expect("Failed to write to temp file");

        let uri = path_to_uri(temp_file.path());

        let schema = json!({
            "type": "object",
            "properties": {
                "user": { "$ref": uri }
            }
        });

        let validator = crate::validator_for(&schema).expect("Schema compilation failed");

        let valid = json!({"user": {"name": "John Doe"}});
        assert!(validator.is_valid(&valid));

        let invalid = json!({"user": {}});
        assert!(!validator.is_valid(&invalid));
    }

    #[test]
    fn test_unknown_scheme() {
        let schema = json!({
            "type": "object",
            "properties": {
                "test": { "$ref": "unknown-schema://test" }
            }
        });

        let result = crate::validator_for(&schema);

        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        assert!(error.contains("Unknown scheme"));
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        assert!(error.contains("External references are not supported on wasm32-unknown-unknown"));
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "resolve-file"))]
    fn create_temp_file(dir: &tempfile::TempDir, name: &str, content: &str) -> String {
        let file_path = dir.path().join(name);
        std::fs::write(&file_path, content).unwrap();
        file_path.to_str().unwrap().to_string()
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_http_retriever_with_default_options() {
        let options = HttpOptions::new();
        let retriever = HttpRetriever::new(&options);
        assert!(retriever.is_ok());
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_http_retriever_nonexistent_cert() {
        let options = HttpOptions::new().add_root_certificate("/nonexistent/cert.pem");
        let result = HttpRetriever::new(&options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpRetrieverError::CertificateRead { .. }));
        assert!(err.to_string().contains("/nonexistent/cert.pem"));
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_http_retriever_error_source() {
        use std::error::Error;

        let options = HttpOptions::new().add_root_certificate("/nonexistent/cert.pem");
        let err = HttpRetriever::new(&options).unwrap_err();
        assert!(err.source().is_some());
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_http_retriever_with_valid_certificate() {
        // Test certificate file generated with:
        // openssl req -x509 -newkey rsa:2048 -keyout /dev/null -out cert.pem -days 3650 -nodes -subj "/CN=test"
        let cert_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../test/foundation/jsonschema/tests/test_cert.pem");
        let options = HttpOptions::new().add_root_certificate(&cert_path);
        let retriever = HttpRetriever::new(&options);
        assert!(retriever.is_ok(), "Failed: {:?}", retriever.err());
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_with_http_options_default() {
        let http_options = HttpOptions::new();
        let schema = json!({"type": "string"});
        let result = crate::options().with_http_options(&http_options);
        assert!(result.is_ok());
        let validator = result.unwrap().build(&schema);
        assert!(validator.is_ok());
        assert!(validator.unwrap().is_valid(&json!("test")));
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_with_http_options_with_timeouts() {
        use std::time::Duration;

        let http_options = HttpOptions::new()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30));
        let schema = json!({"type": "integer"});
        let result = crate::options().with_http_options(&http_options);
        assert!(result.is_ok());
        let validator = result.unwrap().build(&schema).unwrap();
        assert!(validator.is_valid(&json!(42)));
        assert!(!validator.is_valid(&json!("not an integer")));
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_with_http_options_invalid_cert() {
        let http_options = HttpOptions::new().add_root_certificate("/nonexistent/cert.pem");
        let result = crate::options().with_http_options(&http_options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("/nonexistent/cert.pem"));
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_with_http_options_danger_accept_invalid_certs() {
        let http_options = HttpOptions::new().danger_accept_invalid_certs(true);
        let schema = json!({"type": "boolean"});
        let result = crate::options().with_http_options(&http_options);
        assert!(result.is_ok());
        let validator = result.unwrap().build(&schema).unwrap();
        assert!(validator.is_valid(&json!(true)));
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_http_retriever_retrieve_trait() {
        use referencing::Retrieve;
        use std::time::Duration;

        let options = HttpOptions::new().timeout(Duration::from_secs(30));
        let retriever = HttpRetriever::new(&options).unwrap();
        let uri =
            referencing::uri::from_str("https://json-schema.org/draft/2020-12/schema").unwrap();
        let result = retriever.retrieve(&uri);
        assert!(result.is_ok());
        let schema = result.unwrap();
        // The meta-schema should be an object with $schema and $id
        assert!(schema.is_object());
        assert!(schema.get("$schema").is_some());
    }

    #[test]
    #[cfg(all(
        feature = "resolve-http",
        feature = "resolve-file",
        not(target_arch = "wasm32")
    ))]
    fn test_http_retriever_file_scheme() {
        use referencing::Retrieve;

        let dir = tempfile::tempdir().unwrap();
        let schema_content = r#"{"type": "string"}"#;
        let schema_path = dir.path().join("schema.json");
        std::fs::write(&schema_path, schema_content).unwrap();

        let options = HttpOptions::new();
        let retriever = HttpRetriever::new(&options).unwrap();
        let uri = referencing::uri::from_str(&path_to_uri(&schema_path)).unwrap();
        let result = retriever.retrieve(&uri);
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert_eq!(schema, json!({"type": "string"}));
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_http_retriever_unknown_scheme() {
        use referencing::Retrieve;

        let options = HttpOptions::new();
        let retriever = HttpRetriever::new(&options).unwrap();
        let uri = referencing::uri::from_str("ftp://example.com/schema.json").unwrap();
        let result = retriever.retrieve(&uri);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown scheme"));
    }

    #[test]
    #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
    fn test_http_retriever_error_invalid_certificate() {
        use std::io::Write;

        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(b"-----BEGIN CERTIFICATE-----\ninvalid\n-----END CERTIFICATE-----")
            .unwrap();
        temp.flush().unwrap();

        let options = HttpOptions::new().add_root_certificate(temp.path());
        let result = HttpRetriever::new(&options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpRetrieverError::ClientBuild(_)));
        assert!(err.to_string().contains("Failed to build HTTP client"));
    }

    #[tokio::test]
    #[cfg(all(
        feature = "resolve-http",
        feature = "resolve-async",
        not(target_arch = "wasm32")
    ))]
    async fn test_async_http_retriever_retrieve_trait() {
        use referencing::AsyncRetrieve;
        use serde_json::Value;
        use std::time::Duration;

        let options = HttpOptions::new().timeout(Duration::from_secs(30));
        let retriever = AsyncHttpRetriever::new(&options).unwrap();
        let uri =
            referencing::uri::from_str("https://json-schema.org/draft/2020-12/schema").unwrap();
        let result: Result<Value, _> = retriever.retrieve(&uri).await;
        assert!(result.is_ok());
        let schema = result.unwrap();
        // The meta-schema should be an object with $schema and $id
        assert!(schema.is_object());
        assert!(schema.get("$schema").is_some());
    }

    #[tokio::test]
    #[cfg(all(
        feature = "resolve-http",
        feature = "resolve-async",
        feature = "resolve-file",
        not(target_arch = "wasm32")
    ))]
    async fn test_async_http_retriever_file_scheme() {
        use referencing::AsyncRetrieve;
        use serde_json::Value;

        let dir = tempfile::tempdir().unwrap();
        let schema_content = r#"{"type": "integer"}"#;
        let schema_path = dir.path().join("schema.json");
        std::fs::write(&schema_path, schema_content).unwrap();

        let options = HttpOptions::new();
        let retriever = AsyncHttpRetriever::new(&options).unwrap();
        let uri = referencing::uri::from_str(&path_to_uri(&schema_path)).unwrap();
        let result: Result<Value, _> = retriever.retrieve(&uri).await;
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert_eq!(schema, json!({"type": "integer"}));
    }

    #[tokio::test]
    #[cfg(all(
        feature = "resolve-http",
        feature = "resolve-async",
        not(target_arch = "wasm32")
    ))]
    async fn test_async_http_retriever_unknown_scheme() {
        use referencing::AsyncRetrieve;
        use serde_json::Value;

        let options = HttpOptions::new();
        let retriever = AsyncHttpRetriever::new(&options).unwrap();
        let uri = referencing::uri::from_str("ftp://example.com/schema.json").unwrap();
        let result: Result<Value, _> = retriever.retrieve(&uri).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown scheme"));
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), feature = "resolve-file"))]
    fn test_with_base_uri_resolution() {
        let dir = tempfile::tempdir().unwrap();

        let b_schema = r#"
        {
            "type": "object",
            "properties": {
                "age": { "type": "number" }
            },
            "required": ["age"]
        }
        "#;
        let _b_path = create_temp_file(&dir, "b.json", b_schema);

        let a_schema = r#"
        {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$ref": "./b.json",
            "type": "object"
        }
        "#;
        let a_path = create_temp_file(&dir, "a.json", a_schema);

        let valid_instance = serde_json::json!({ "age": 30 });

        let schema_str = std::fs::read_to_string(&a_path).unwrap();
        let schema_json: serde_json::Value = serde_json::from_str(&schema_str).unwrap();

        let base_uri = path_to_uri(dir.path());
        let validator = crate::options()
            .with_base_uri(format!("{base_uri}/"))
            .build(&schema_json)
            .expect("Schema compilation failed");

        assert!(validator.is_valid(&valid_instance));

        let invalid_instance = serde_json::json!({ "age": "thirty" });
        assert!(!validator.is_valid(&invalid_instance));
    }
}
