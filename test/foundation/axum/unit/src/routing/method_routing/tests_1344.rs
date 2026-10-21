#[cfg(test)]
mod tests {
    use super::*;
    use crate::{extract::State, handler::HandlerWithoutStateExt};
    use http::{header::ALLOW, HeaderMap};
    use http_body::BodyExt;
    use std::time::Duration;
    use tower::ServiceExt;
    use tower_http::{
        services::fs::ServeDir, timeout::TimeoutLayer, validate_request::ValidateRequestHeaderLayer,
    };

    #[crate::test]
    async fn method_not_allowed_by_default() {
        let mut svc = MethodRouter::new();
        let (status, _, body) = call(Method::GET, &mut svc).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert!(body.is_empty());
    }

    #[crate::test]
    async fn get_service_fn() {
        async fn handle(_req: Request) -> Result<Response<Body>, Infallible> {
            Ok(Response::new(Body::from("ok")))
        }

        let mut svc = get_service(service_fn(handle));

        let (status, _, body) = call(Method::GET, &mut svc).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "ok");
    }

    #[crate::test]
    async fn get_handler() {
        let mut svc = MethodRouter::new().get(ok);
        let (status, _, body) = call(Method::GET, &mut svc).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "ok");
    }

    #[crate::test]
    async fn get_accepts_head() {
        let mut svc = MethodRouter::new().get(ok);
        let (status, _, body) = call(Method::HEAD, &mut svc).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.is_empty());
    }

    #[crate::test]
    async fn head_takes_precedence_over_get() {
        let mut svc = MethodRouter::new().head(created).get(ok);
        let (status, _, body) = call(Method::HEAD, &mut svc).await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(body.is_empty());
    }

    #[crate::test]
    async fn merge() {
        let mut svc = get(ok).merge(post(ok)).merge(connect(ok));

        let (status, _, _) = call(Method::GET, &mut svc).await;
        assert_eq!(status, StatusCode::OK);

        let (status, _, _) = call(Method::POST, &mut svc).await;
        assert_eq!(status, StatusCode::OK);

        let (status, _, _) = call(Method::CONNECT, &mut svc).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[allow(deprecated)]
    #[crate::test]
    async fn layer() {
        let mut svc = MethodRouter::new()
            .get(|| async { std::future::pending::<()>().await })
            .layer(ValidateRequestHeaderLayer::bearer("password"));

        // method with route
        let (status, _, _) = call(Method::GET, &mut svc).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        // method without route
        let (status, _, _) = call(Method::DELETE, &mut svc).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[allow(deprecated)]
    #[crate::test]
    async fn route_layer() {
        let mut svc = MethodRouter::new()
            .get(|| async { std::future::pending::<()>().await })
            .route_layer(ValidateRequestHeaderLayer::bearer("password"));

        // method with route
        let (status, _, _) = call(Method::GET, &mut svc).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        // method without route
        let (status, _, _) = call(Method::DELETE, &mut svc).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[allow(dead_code, deprecated)]
    async fn building_complex_router() {
        let app = crate::Router::new().route(
            "/",
            // use the all the things 💣️
            get(ok)
                .post(ok)
                .route_layer(ValidateRequestHeaderLayer::bearer("password"))
                .merge(delete_service(ServeDir::new(".")))
                .fallback(|| async { StatusCode::NOT_FOUND })
                .put(ok)
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(10),
                )),
        );

        let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
        crate::serve(listener, app).await.unwrap();
    }

    #[crate::test]
    async fn sets_allow_header() {
        let mut svc = MethodRouter::new().put(ok).patch(ok);
        let (status, headers, _) = call(Method::GET, &mut svc).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(headers[ALLOW], "PUT,PATCH");
    }

    #[crate::test]
    async fn sets_allow_header_get_head() {
        let mut svc = MethodRouter::new().get(ok).head(ok);
        let (status, headers, _) = call(Method::PUT, &mut svc).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(headers[ALLOW], "GET,HEAD");
    }

    #[crate::test]
    async fn empty_allow_header_by_default() {
        let mut svc = MethodRouter::new();
        let (status, headers, _) = call(Method::PATCH, &mut svc).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(headers[ALLOW], "");
    }

    #[crate::test]
    async fn allow_header_when_merging() {
        let a = put(ok).patch(ok);
        let b = get(ok).head(ok);
        let mut svc = a.merge(b);

        let (status, headers, _) = call(Method::DELETE, &mut svc).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(headers[ALLOW], "PUT,PATCH,GET,HEAD");
    }

    #[crate::test]
    async fn allow_header_any() {
        let mut svc = any(ok);

        let (status, headers, _) = call(Method::GET, &mut svc).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!headers.contains_key(ALLOW));
    }

    #[crate::test]
    async fn allow_header_with_fallback() {
        let mut svc = MethodRouter::new()
            .get(ok)
            .fallback(|| async { (StatusCode::METHOD_NOT_ALLOWED, "Method not allowed") });

        let (status, headers, _) = call(Method::DELETE, &mut svc).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(headers[ALLOW], "GET,HEAD");
    }

    #[crate::test]
    async fn allow_header_with_fallback_that_sets_allow() {
        async fn fallback(method: Method) -> Response {
            if method == Method::POST {
                "OK".into_response()
            } else {
                (
                    StatusCode::METHOD_NOT_ALLOWED,
                    [(ALLOW, "GET,POST")],
                    "Method not allowed",
                )
                    .into_response()
            }
        }

        let mut svc = MethodRouter::new().get(ok).fallback(fallback);

        let (status, _, _) = call(Method::GET, &mut svc).await;
        assert_eq!(status, StatusCode::OK);

        let (status, _, _) = call(Method::POST, &mut svc).await;
        assert_eq!(status, StatusCode::OK);

        let (status, headers, _) = call(Method::DELETE, &mut svc).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(headers[ALLOW], "GET,POST");
    }

    #[crate::test]
    async fn allow_header_noop_middleware() {
        let mut svc = MethodRouter::new()
            .get(ok)
            .layer(tower::layer::util::Identity::new());

        let (status, headers, _) = call(Method::DELETE, &mut svc).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(headers[ALLOW], "GET,HEAD");
    }

    #[crate::test]
    #[should_panic(
        expected = "Overlapping method route. Cannot add two method routes that both handle `GET`"
    )]
    async fn handler_overlaps() {
        let _: MethodRouter<()> = get(ok).get(ok);
    }

    #[crate::test]
    #[should_panic(
        expected = "Overlapping method route. Cannot add two method routes that both handle `POST`"
    )]
    async fn service_overlaps() {
        let _: MethodRouter<()> = post_service(ok.into_service()).post_service(ok.into_service());
    }

    #[crate::test]
    async fn get_head_does_not_overlap() {
        let _: MethodRouter<()> = get(ok).head(ok);
    }

    #[crate::test]
    async fn head_get_does_not_overlap() {
        let _: MethodRouter<()> = head(ok).get(ok);
    }

    #[crate::test]
    async fn accessing_state() {
        let mut svc = MethodRouter::new()
            .get(|State(state): State<&'static str>| async move { state })
            .with_state("state");

        let (status, _, text) = call(Method::GET, &mut svc).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(text, "state");
    }

    #[crate::test]
    async fn fallback_accessing_state() {
        let mut svc = MethodRouter::new()
            .fallback(|State(state): State<&'static str>| async move { state })
            .with_state("state");

        let (status, _, text) = call(Method::GET, &mut svc).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(text, "state");
    }

    #[crate::test]
    async fn merge_accessing_state() {
        let one = get(|State(state): State<&'static str>| async move { state });
        let two = post(|State(state): State<&'static str>| async move { state });

        let mut svc = one.merge(two).with_state("state");

        let (status, _, text) = call(Method::GET, &mut svc).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(text, "state");

        let (status, _, _) = call(Method::POST, &mut svc).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(text, "state");
    }

    async fn call<S>(method: Method, svc: &mut S) -> (StatusCode, HeaderMap, String)
    where
        S: Service<Request, Error = Infallible>,
        S::Response: IntoResponse,
    {
        let request = Request::builder()
            .uri("/")
            .method(method)
            .body(Body::empty())
            .unwrap();
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap()
            .into_response();
        let (parts, body) = response.into_parts();
        let body =
            String::from_utf8(BodyExt::collect(body).await.unwrap().to_bytes().to_vec()).unwrap();
        (parts.status, parts.headers, body)
    }

    async fn ok() -> (StatusCode, &'static str) {
        (StatusCode::OK, "ok")
    }

    async fn created() -> (StatusCode, &'static str) {
        (StatusCode::CREATED, "created")
    }
}
