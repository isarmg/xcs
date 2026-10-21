#[cfg(test)]
mod tests {
    use super::*;
    use crate::{routing::get, test_helpers::TestClient, Router};
    use http::{HeaderMap, StatusCode};

    #[crate::test]
    async fn works() {
        async fn add_header<B>(mut req: Request<B>) -> Request<B> {
            req.headers_mut().insert("x-foo", "foo".parse().unwrap());
            req
        }

        async fn handler(headers: HeaderMap) -> Response {
            headers["x-foo"]
                .to_str()
                .unwrap()
                .to_owned()
                .into_response()
        }

        let app = Router::new()
            .route("/", get(handler))
            .layer(map_request(add_header));
        let client = TestClient::new(app);

        let res = client.get("/").await;

        assert_eq!(res.text().await, "foo");
    }

    #[crate::test]
    async fn works_for_short_circutting() {
        async fn add_header<B>(_req: Request<B>) -> Result<Request<B>, (StatusCode, &'static str)> {
            Err((StatusCode::INTERNAL_SERVER_ERROR, "something went wrong"))
        }

        async fn handler(_headers: HeaderMap) -> Response {
            unreachable!()
        }

        let app = Router::new()
            .route("/", get(handler))
            .layer(map_request(add_header));
        let client = TestClient::new(app);

        let res = client.get("/").await;

        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(res.text().await, "something went wrong");
    }
}
