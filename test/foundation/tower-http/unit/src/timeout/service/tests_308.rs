#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::Body;
    use http::{Request, Response, StatusCode};
    use std::time::Duration;
    use service_stack::{BoxError, ServiceBuilder, ServiceExt};

    #[tokio::test]
    async fn request_completes_within_timeout() {
        let mut service = ServiceBuilder::new()
            .layer(TimeoutLayer::with_status_code(
                StatusCode::GATEWAY_TIMEOUT,
                Duration::from_secs(1),
            ))
            .service_fn(fast_handler);

        let request = Request::get("/").body(Body::empty()).unwrap();
        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn timeout_middleware_with_custom_status_code() {
        let timeout_service = Timeout::with_status_code(
            service_stack::service_fn(slow_handler),
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_millis(10),
        );

        let mut service = ServiceBuilder::new().service(timeout_service);

        let request = Request::get("/").body(Body::empty()).unwrap();
        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::REQUEST_TIMEOUT);
    }

    #[tokio::test]
    async fn timeout_response_has_empty_body() {
        let mut service = ServiceBuilder::new()
            .layer(TimeoutLayer::with_status_code(
                StatusCode::GATEWAY_TIMEOUT,
                Duration::from_millis(10),
            ))
            .service_fn(slow_handler);

        let request = Request::get("/").body(Body::empty()).unwrap();
        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::GATEWAY_TIMEOUT);

        // Verify the body is empty (default)
        use http_body::BodyExt;
        let body = res.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        assert!(bytes.is_empty());
    }

    #[tokio::test]
    async fn deprecated_new_method_compatibility() {
        #[allow(deprecated)]
        let layer = TimeoutLayer::new(Duration::from_millis(10));

        let mut service = ServiceBuilder::new().layer(layer).service_fn(slow_handler);

        let request = Request::get("/").body(Body::empty()).unwrap();
        let res = service.ready().await.unwrap().call(request).await.unwrap();

        // Should use default 408 status code
        assert_eq!(res.status(), StatusCode::REQUEST_TIMEOUT);
    }

    async fn slow_handler(_req: Request<Body>) -> Result<Response<Body>, BoxError> {
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap())
    }

    async fn fast_handler(_req: Request<Body>) -> Result<Response<Body>, BoxError> {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap())
    }
}
