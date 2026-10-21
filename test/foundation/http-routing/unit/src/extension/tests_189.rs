#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::get;
    use crate::test_helpers::TestClient;
    use crate::Router;
    use http::StatusCode;

    #[derive(Clone)]
    struct Foo(String);

    #[derive(Clone)]
    struct Bar(String);

    #[crate::test]
    async fn extension_extractor() {
        async fn requires_foo(Extension(foo): Extension<Foo>) -> String {
            foo.0
        }

        async fn optional_foo(extension: Option<Extension<Foo>>) -> String {
            extension.map(|foo| foo.0 .0).unwrap_or("none".to_owned())
        }

        async fn requires_bar(Extension(bar): Extension<Bar>) -> String {
            bar.0
        }

        async fn optional_bar(extension: Option<Extension<Bar>>) -> String {
            extension.map(|bar| bar.0 .0).unwrap_or("none".to_owned())
        }

        let app = Router::new()
            .route("/requires_foo", get(requires_foo))
            .route("/optional_foo", get(optional_foo))
            .route("/requires_bar", get(requires_bar))
            .route("/optional_bar", get(optional_bar))
            .layer(Extension(Foo("foo".to_owned())));

        let client = TestClient::new(app);

        let response = client.get("/requires_foo").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.text().await, "foo");

        let response = client.get("/optional_foo").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.text().await, "foo");

        let response = client.get("/requires_bar").await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.text().await, "Missing request extension: Extension of type `http_routing::extension::tests::Bar` was not found. Perhaps you forgot to add it? See `http_routing::Extension`.");

        let response = client.get("/optional_bar").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.text().await, "none");
    }
}
