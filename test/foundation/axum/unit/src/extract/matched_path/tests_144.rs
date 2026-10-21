#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        extract::Request,
        handler::HandlerWithoutStateExt,
        middleware::map_request,
        routing::{any, get},
        test_helpers::*,
        Router,
    };
    use http::StatusCode;

    #[crate::test]
    async fn extracting_on_handler() {
        let app = Router::new().route(
            "/{a}",
            get(|path: MatchedPath| async move { path.as_str().to_owned() }),
        );

        let client = TestClient::new(app);

        let res = client.get("/foo").await;
        assert_eq!(res.text().await, "/{a}");
    }

    #[crate::test]
    async fn extracting_on_handler_in_nested_router() {
        let app = Router::new().nest(
            "/{a}",
            Router::new().route(
                "/{b}",
                get(|path: MatchedPath| async move { path.as_str().to_owned() }),
            ),
        );

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.text().await, "/{a}/{b}");
    }

    #[crate::test]
    async fn extracting_on_handler_in_deeply_nested_router() {
        let app = Router::new().nest(
            "/{a}",
            Router::new().nest(
                "/{b}",
                Router::new().route(
                    "/{c}",
                    get(|path: MatchedPath| async move { path.as_str().to_owned() }),
                ),
            ),
        );

        let client = TestClient::new(app);

        let res = client.get("/foo/bar/baz").await;
        assert_eq!(res.text().await, "/{a}/{b}/{c}");
    }

    #[crate::test]
    async fn cannot_extract_nested_matched_path_in_middleware() {
        async fn extract_matched_path<B>(
            matched_path: Option<MatchedPath>,
            req: Request<B>,
        ) -> Request<B> {
            assert!(matched_path.is_none());
            req
        }

        let app = Router::new()
            .nest_service("/{a}", Router::new().route("/{b}", get(|| async move {})))
            .layer(map_request(extract_matched_path));

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn can_extract_nested_matched_path_in_middleware_using_nest() {
        async fn extract_matched_path<B>(
            matched_path: Option<MatchedPath>,
            req: Request<B>,
        ) -> Request<B> {
            assert_eq!(matched_path.unwrap().as_str(), "/{a}/{b}");
            req
        }

        let app = Router::new()
            .nest("/{a}", Router::new().route("/{b}", get(|| async move {})))
            .layer(map_request(extract_matched_path));

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn cannot_extract_nested_matched_path_in_middleware_via_extension() {
        async fn assert_no_matched_path<B>(req: Request<B>) -> Request<B> {
            assert!(req.extensions().get::<MatchedPath>().is_none());
            req
        }

        let app = Router::new()
            .nest_service("/{a}", Router::new().route("/{b}", get(|| async move {})))
            .layer(map_request(assert_no_matched_path));

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn can_extract_nested_matched_path_in_middleware_via_extension_using_nest() {
        async fn assert_matched_path<B>(req: Request<B>) -> Request<B> {
            assert!(req.extensions().get::<MatchedPath>().is_some());
            req
        }

        let app = Router::new()
            .nest("/{a}", Router::new().route("/{b}", get(|| async move {})))
            .layer(map_request(assert_matched_path));

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn can_extract_nested_matched_path_in_middleware_on_nested_router() {
        async fn extract_matched_path<B>(matched_path: MatchedPath, req: Request<B>) -> Request<B> {
            assert_eq!(matched_path.as_str(), "/{a}/{b}");
            req
        }

        let app = Router::new().nest(
            "/{a}",
            Router::new()
                .route("/{b}", get(|| async move {}))
                .layer(map_request(extract_matched_path)),
        );

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn can_extract_nested_matched_path_in_middleware_on_nested_router_via_extension() {
        async fn extract_matched_path<B>(req: Request<B>) -> Request<B> {
            let matched_path = req.extensions().get::<MatchedPath>().unwrap();
            assert_eq!(matched_path.as_str(), "/{a}/{b}");
            req
        }

        let app = Router::new().nest(
            "/{a}",
            Router::new()
                .route("/{b}", get(|| async move {}))
                .layer(map_request(extract_matched_path)),
        );

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn extracting_on_nested_handler() {
        async fn handler(path: Option<MatchedPath>) {
            assert!(path.is_none());
        }

        let app = Router::new().nest_service("/{a}", handler.into_service());

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    // https://github.com/tokio-rs/axum/issues/1579
    #[crate::test]
    async fn doesnt_panic_if_router_called_from_wildcard_route() {
        use tower::ServiceExt;

        let app = Router::new().route(
            "/{*path}",
            any(|req: Request| {
                Router::new()
                    .nest("/foo", Router::new().route("/bar", get(|| async {})))
                    .oneshot(req)
            }),
        );

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn cant_extract_in_fallback() {
        async fn handler(path: Option<MatchedPath>, req: Request) {
            assert!(path.is_none());
            assert!(req.extensions().get::<MatchedPath>().is_none());
        }

        let app = Router::new().fallback(handler);

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn matching_colon() {
        let app = Router::new().without_v07_checks().route(
            "/:foo",
            get(|path: MatchedPath| async move { path.as_str().to_owned() }),
        );

        let client = TestClient::new(app);

        let res = client.get("/:foo").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "/:foo");

        let res = client.get("/:bar").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        let res = client.get("/foo").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[crate::test]
    async fn matching_asterisk() {
        let app = Router::new().without_v07_checks().route(
            "/*foo",
            get(|path: MatchedPath| async move { path.as_str().to_owned() }),
        );

        let client = TestClient::new(app);

        let res = client.get("/*foo").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "/*foo");

        let res = client.get("/*bar").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        let res = client.get("/foo").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
