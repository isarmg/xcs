#[cfg(all(test, feature = "resolve-async", not(target_arch = "wasm32")))]
mod async_tests {
    use super::*;
    use crate::Registry;
    use serde_json::json;
    #[cfg(feature = "resolve-file")]
    use std::io::Write;

    #[tokio::test]
    #[cfg(feature = "resolve-file")]
    async fn test_async_retrieve_from_file() {
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

        let validator = crate::async_options()
            .with_base_uri("http://example.com/schema")
            .with_retriever(DefaultRetriever)
            .build(&schema)
            .await
            .expect("Invalid schema");

        let valid = json!({"user": {"name": "John Doe"}});
        assert!(validator.is_valid(&valid));

        let invalid = json!({"user": {}});
        assert!(!validator.is_valid(&invalid));
    }

    #[tokio::test]
    async fn test_async_unknown_scheme() {
        let schema = json!({
            "type": "object",
            "properties": {
                "test": { "$ref": "unknown-schema://test" }
            }
        });

        let result = Registry::new()
            .async_retriever(DefaultRetriever)
            .add("http://example.com/schema", schema)
            .expect("Resource should be accepted")
            .async_prepare()
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Unknown scheme"));
    }

    #[tokio::test]
    #[cfg(feature = "resolve-file")]
    async fn test_async_concurrent_retrievals() {
        let mut temp_files = vec![];
        let mut uris = vec![];

        // Create multiple temp files with different schemas
        for i in 0..3 {
            let mut temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
            let schema = json!({
                "type": "object",
                "properties": {
                    "field": { "type": "string", "minLength": i }
                }
            });
            write!(temp_file, "{schema}").expect("Failed to write to temp file");
            uris.push(path_to_uri(temp_file.path()));
            temp_files.push(temp_file);
        }

        // Create a schema that references all temp files
        let schema = json!({
            "type": "object",
            "properties": {
                "obj1": { "$ref": uris[0] },
                "obj2": { "$ref": uris[1] },
                "obj3": { "$ref": uris[2] }
            }
        });

        let validator = crate::async_options()
            .with_base_uri("http://example.com/schema")
            .with_retriever(DefaultRetriever)
            .build(&schema)
            .await
            .expect("Invalid schema");

        let valid = json!({
            "obj1": { "field": "" },      // minLength: 0
            "obj2": { "field": "a" },     // minLength: 1
            "obj3": { "field": "ab" }     // minLength: 2
        });
        assert!(validator.is_valid(&valid));

        // Test invalid data
        let invalid = json!({
            "obj1": { "field": "" },
            "obj2": { "field": "" },      // should be at least 1 char
            "obj3": { "field": "a" }      // should be at least 2 chars
        });
        assert!(!validator.is_valid(&invalid));
    }
}
