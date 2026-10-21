#[cfg(test)]
mod tests {
    use axum_core::response::Response;
    use http::StatusCode;

    use crate::{
        extract::{NestedPath, Request},
        middleware::{from_fn, Next},
        routing::get,
        test_helpers::*,
        Router,
    };

    #[crate::test]
    async fn one_level_of_nesting() {
        let api = Router::new().route(
            "/users",
            get(|nested_path: NestedPath| {
                assert_eq!(nested_path.as_str(), "/api");
                async {}
            }),
        );

        let app = Router::new().nest("/api", api);

        let client = TestClient::new(app);

        let res = client.get("/api/users").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn one_level_of_nesting_with_trailing_slash() {
        let api = Router::new().route(
            "/users",
            get(|nested_path: NestedPath| {
                assert_eq!(nested_path.as_str(), "/api/");
                async {}
            }),
        );

        let app = Router::new().nest("/api/", api);

        let client = TestClient::new(app);

        let res = client.get("/api/users").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn two_levels_of_nesting() {
        let api = Router::new().route(
            "/users",
            get(|nested_path: NestedPath| {
                assert_eq!(nested_path.as_str(), "/api/v2");
                async {}
            }),
        );

        let app = Router::new().nest("/api", Router::new().nest("/v2", api));

        let client = TestClient::new(app);

        let res = client.get("/api/v2/users").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn two_levels_of_nesting_with_trailing_slash() {
        let api = Router::new().route(
            "/users",
            get(|nested_path: NestedPath| {
                assert_eq!(nested_path.as_str(), "/api/v2");
                async {}
            }),
        );

        let app = Router::new().nest("/api/", Router::new().nest("/v2", api));

        let client = TestClient::new(app);

        let res = client.get("/api/v2/users").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn in_fallbacks() {
        let api = Router::new().fallback(get(|nested_path: NestedPath| {
            assert_eq!(nested_path.as_str(), "/api");
            async {}
        }));

        let app = Router::new().nest("/api", api);

        let client = TestClient::new(app);

        let res = client.get("/api/doesnt-exist").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn in_middleware() {
        async fn middleware(nested_path: NestedPath, req: Request, next: Next) -> Response {
            assert_eq!(nested_path.as_str(), "/api");
            next.run(req).await
        }

        let api = Router::new()
            .route("/users", get(|| async {}))
            .layer(from_fn(middleware));

        let app = Router::new().nest("/api", api);

        let client = TestClient::new(app);

        let res = client.get("/api/users").await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}
