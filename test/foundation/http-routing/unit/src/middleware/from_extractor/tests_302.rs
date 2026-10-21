#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handler::Handler, routing::get, test_helpers::*, Router};
    use http_routing::kernel::extract::FromRef;
    use http::{header, request::Parts, StatusCode};
    use tower_http::limit::RequestBodyLimitLayer;

    #[crate::test]
    async fn test_from_extractor() {
        #[derive(Clone)]
        struct Secret(&'static str);

        struct RequireAuth;

        impl<S> FromRequestParts<S> for RequireAuth
        where
            S: Send + Sync,
            Secret: FromRef<S>,
        {
            type Rejection = StatusCode;

            async fn from_request_parts(
                parts: &mut Parts,
                state: &S,
            ) -> Result<Self, Self::Rejection> {
                let Secret(secret) = Secret::from_ref(state);
                if let Some(auth) = parts
                    .headers
                    .get(header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                {
                    if auth == secret {
                        return Ok(Self);
                    }
                }

                Err(StatusCode::UNAUTHORIZED)
            }
        }

        async fn handler() {}

        let state = Secret("secret");
        let app = Router::new().route(
            "/",
            get(handler.layer(from_extractor_with_state::<RequireAuth, _>(state))),
        );

        let client = TestClient::new(app);

        let res = client.get("/").await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        let res = client
            .get("/")
            .header(http::header::AUTHORIZATION, "secret")
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    // just needs to compile
    #[allow(dead_code)]
    fn works_with_request_body_limit() {
        struct MyExtractor;

        impl<S> FromRequestParts<S> for MyExtractor
        where
            S: Send + Sync,
        {
            type Rejection = std::convert::Infallible;

            async fn from_request_parts(
                _parts: &mut Parts,
                _state: &S,
            ) -> Result<Self, Self::Rejection> {
                unimplemented!()
            }
        }

        let _: Router = Router::new()
            .layer(from_extractor::<MyExtractor>())
            .layer(RequestBodyLimitLayer::new(1));
    }
}
