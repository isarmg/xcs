#[cfg(test)]
mod tests {
    use super::*;
    use crate::{body::Body, routing::get, Router};
    use http::{HeaderMap, StatusCode};
    use http_body::BodyExt;
    use tower::ServiceExt;

    #[crate::test]
    async fn basic() {
        async fn insert_header(mut req: Request, next: Next) -> impl IntoResponse {
            req.headers_mut()
                .insert("x-axum-test", "ok".parse().unwrap());

            next.run(req).await
        }

        async fn handle(headers: HeaderMap) -> String {
            headers["x-axum-test"].to_str().unwrap().to_owned()
        }

        let app = Router::new()
            .route("/", get(handle))
            .layer(from_fn(insert_header));

        let res = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }
}
