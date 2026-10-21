#[cfg(test)]
mod tests {
    use http_routing::kernel::extract::DefaultBodyLimit;

    use super::*;
    use crate::{routing::post, test_helpers::*, Router};

    #[crate::test]
    async fn content_type_with_encoding() {
        const BYTES: &[u8] = "<!doctype html><title>🦀</title>".as_bytes();
        const FILE_NAME: &str = "index.html";
        const CONTENT_TYPE: &str = "text/html; charset=utf-8";

        async fn handle(mut multipart: Multipart) -> impl IntoResponse {
            let field = multipart.next_field().await.unwrap().unwrap();

            assert_eq!(field.file_name().unwrap(), FILE_NAME);
            assert_eq!(field.content_type().unwrap(), CONTENT_TYPE);
            assert_eq!(field.headers()["foo"], "bar");
            assert_eq!(field.bytes().await.unwrap(), BYTES);

            assert!(multipart.next_field().await.unwrap().is_none());
        }

        let app = Router::new().route("/", post(handle));

        let client = TestClient::new(app);

        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(BYTES)
                .file_name(FILE_NAME)
                .mime_str(CONTENT_TYPE)
                .unwrap()
                .headers(reqwest::header::HeaderMap::from_iter([(
                    reqwest::header::HeaderName::from_static("foo"),
                    reqwest::header::HeaderValue::from_static("bar"),
                )])),
        );

        client.post("/").multipart(form).await;
    }

    // No need for this to be a #[test], we just want to make sure it compiles
    fn _multipart_from_request_limited() {
        async fn handler(_: Multipart) {}
        let _app: Router = Router::new()
            .route("/", post(handler))
            .layer(tower_http::limit::RequestBodyLimitLayer::new(1024));
    }

    #[crate::test]
    async fn body_too_large() {
        const BYTES: &[u8] = "<!doctype html><title>🦀</title>".as_bytes();

        async fn handle(mut multipart: Multipart) -> Result<(), MultipartError> {
            while let Some(field) = multipart.next_field().await? {
                field.bytes().await?;
            }
            Ok(())
        }

        let app = Router::new()
            .route("/", post(handle))
            .layer(DefaultBodyLimit::max(BYTES.len() - 1));

        let client = TestClient::new(app);

        let form =
            reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(BYTES));

        let res = client.post("/").multipart(form).await;
        assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(res.text().await, "Request payload is too large");
    }

    #[crate::test]
    async fn optional_multipart() {
        const BYTES: &[u8] = "<!doctype html><title>🦀</title>".as_bytes();

        async fn handle(multipart: Option<Multipart>) -> Result<StatusCode, MultipartError> {
            if let Some(mut multipart) = multipart {
                while let Some(field) = multipart.next_field().await? {
                    field.bytes().await?;
                }
                Ok(StatusCode::OK)
            } else {
                Ok(StatusCode::NO_CONTENT)
            }
        }

        let app = Router::new().route("/", post(handle));
        let client = TestClient::new(app);
        let form =
            reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(BYTES));

        let res = client.post("/").multipart(form).await;
        assert_eq!(res.status(), StatusCode::OK);

        let res = client.post("/").await;
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }
}
