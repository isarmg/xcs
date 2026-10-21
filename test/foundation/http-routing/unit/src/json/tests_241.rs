#[cfg(test)]
mod tests {
    use super::*;
    use crate::{routing::post, test_helpers::*, Router};
    use data_serialization::Deserialize;
    use json_codec::{json, Value};

    #[crate::test]
    async fn deserialize_body() {
        #[derive(Debug, Deserialize)]
        struct Input {
            foo: String,
        }

        let app = Router::new().route("/", post(|input: Json<Input>| async { input.0.foo }));

        let client = TestClient::new(app);
        let res = client.post("/").json(&json!({ "foo": "bar" })).await;
        let body = res.text().await;

        assert_eq!(body, "bar");
    }

    #[crate::test]
    async fn consume_body_to_json_requires_json_content_type() {
        #[derive(Debug, Deserialize)]
        struct Input {
            foo: String,
        }

        let app = Router::new().route("/", post(|input: Json<Input>| async { input.0.foo }));

        let client = TestClient::new(app);
        let res = client.post("/").body(r#"{ "foo": "bar" }"#).await;

        let status = res.status();

        assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[crate::test]
    async fn json_content_types() {
        async fn valid_json_content_type(content_type: &str) -> bool {
            println!("testing {content_type:?}");

            let app = Router::new().route("/", post(|Json(_): Json<Value>| async {}));

            let res = TestClient::new(app)
                .post("/")
                .header("content-type", content_type)
                .body("{}")
                .await;

            res.status() == StatusCode::OK
        }

        assert!(valid_json_content_type("application/json").await);
        assert!(valid_json_content_type("application/json; charset=utf-8").await);
        assert!(valid_json_content_type("application/json;charset=utf-8").await);
        assert!(valid_json_content_type("application/cloudevents+json").await);
        assert!(!valid_json_content_type("text/json").await);
    }

    #[crate::test]
    async fn invalid_json_syntax() {
        let app = Router::new().route("/", post(|_: Json<json_codec::Value>| async {}));

        let client = TestClient::new(app);
        let res = client
            .post("/")
            .body("{")
            .header("content-type", "application/json")
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[crate::test]
    async fn extra_chars_after_valid_json_syntax() {
        #[derive(Debug, Deserialize)]
        struct Input {
            foo: String,
        }

        let app = Router::new().route("/", post(|input: Json<Input>| async { input.0.foo }));

        let client = TestClient::new(app);
        let res = client
            .post("/")
            .body(r#"{ "foo": "bar" } baz "#)
            .header("content-type", "application/json")
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body_text = res.text().await;
        assert_eq!(
            body_text,
            "Failed to parse the request body as JSON: trailing characters at line 1 column 18"
        );
    }

    #[derive(Deserialize)]
    struct Foo {
        #[allow(dead_code)]
        a: i32,
        #[allow(dead_code)]
        b: Vec<Bar>,
    }

    #[derive(Deserialize)]
    struct Bar {
        #[allow(dead_code)]
        x: i32,
        #[allow(dead_code)]
        y: i32,
    }

    #[crate::test]
    async fn invalid_json_data() {
        let app = Router::new().route("/", post(|_: Json<Foo>| async {}));

        let client = TestClient::new(app);
        let res = client
            .post("/")
            .body("{\"a\": 1, \"b\": [{\"x\": 2}]}")
            .header("content-type", "application/json")
            .await;

        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body_text = res.text().await;
        assert_eq!(
            body_text,
            "Failed to deserialize the JSON body into the target type: b[0]: missing field `y` at line 1 column 23"
        );
    }
}
