#[cfg(test)]
mod tests {
    use crate::validate_request::ValidateRequestHeaderLayer;

    #[allow(unused_imports)]
    use super::*;
    use crate::test_helpers::Body;
    use http::header;
    use service_stack::{BoxError, ServiceBuilder, ServiceExt};
    use service_stack::Service;

    #[tokio::test]
    async fn valid_basic_token() {
        let mut service = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::basic("foo", "bar"))
            .service_fn(echo);

        let request = Request::get("/")
            .header(
                header::AUTHORIZATION,
                format!("Basic {}", BASE64.encode("foo:bar")),
            )
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn invalid_basic_token() {
        let mut service = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::basic("foo", "bar"))
            .service_fn(echo);

        let request = Request::get("/")
            .header(
                header::AUTHORIZATION,
                format!("Basic {}", BASE64.encode("wrong:credentials")),
            )
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        let www_authenticate = res.headers().get(header::WWW_AUTHENTICATE).unwrap();
        assert_eq!(www_authenticate, "Basic");
    }

    #[tokio::test]
    async fn valid_bearer_token() {
        let mut service = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::bearer("foobar"))
            .service_fn(echo);

        let request = Request::get("/")
            .header(header::AUTHORIZATION, "Bearer foobar")
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn basic_auth_is_case_sensitive_in_prefix() {
        let mut service = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::basic("foo", "bar"))
            .service_fn(echo);

        let request = Request::get("/")
            .header(
                header::AUTHORIZATION,
                format!("basic {}", BASE64.encode("foo:bar")),
            )
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn basic_auth_is_case_sensitive_in_value() {
        let mut service = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::basic("foo", "bar"))
            .service_fn(echo);

        let request = Request::get("/")
            .header(
                header::AUTHORIZATION,
                format!("Basic {}", BASE64.encode("Foo:bar")),
            )
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn invalid_bearer_token() {
        let mut service = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::bearer("foobar"))
            .service_fn(echo);

        let request = Request::get("/")
            .header(header::AUTHORIZATION, "Bearer wat")
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bearer_token_is_case_sensitive_in_prefix() {
        let mut service = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::bearer("foobar"))
            .service_fn(echo);

        let request = Request::get("/")
            .header(header::AUTHORIZATION, "bearer foobar")
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bearer_token_is_case_sensitive_in_token() {
        let mut service = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::bearer("foobar"))
            .service_fn(echo);

        let request = Request::get("/")
            .header(header::AUTHORIZATION, "Bearer Foobar")
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    async fn echo(req: Request<Body>) -> Result<Response<Body>, BoxError> {
        Ok(Response::new(req.into_body()))
    }
}
