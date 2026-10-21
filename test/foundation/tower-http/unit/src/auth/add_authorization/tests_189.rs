#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::Body;
    use crate::validate_request::ValidateRequestHeaderLayer;
    use http::{Response, StatusCode};
    use std::convert::Infallible;
    use tower::{BoxError, Service, ServiceBuilder, ServiceExt};

    #[tokio::test]
    async fn basic() {
        // service that requires auth for all requests
        let svc = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::basic("foo", "bar"))
            .service_fn(echo);

        // make a client that adds auth
        let mut client = AddAuthorization::basic(svc, "foo", "bar");

        let res = client
            .ready()
            .await
            .unwrap()
            .call(Request::new(Body::empty()))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn token() {
        // service that requires auth for all requests
        let svc = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::bearer("foo"))
            .service_fn(echo);

        // make a client that adds auth
        let mut client = AddAuthorization::bearer(svc, "foo");

        let res = client
            .ready()
            .await
            .unwrap()
            .call(Request::new(Body::empty()))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn making_header_sensitive() {
        let svc = ServiceBuilder::new()
            .layer(ValidateRequestHeaderLayer::bearer("foo"))
            .service_fn(|request: Request<Body>| async move {
                let auth = request.headers().get(http::header::AUTHORIZATION).unwrap();
                assert!(auth.is_sensitive());

                Ok::<_, Infallible>(Response::new(Body::empty()))
            });

        let mut client = AddAuthorization::bearer(svc, "foo").as_sensitive(true);

        let res = client
            .ready()
            .await
            .unwrap()
            .call(Request::new(Body::empty()))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    async fn echo(req: Request<Body>) -> Result<Response<Body>, BoxError> {
        Ok(Response::new(req.into_body()))
    }
}
