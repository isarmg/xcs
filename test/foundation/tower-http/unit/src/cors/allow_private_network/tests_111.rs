#[cfg(test)]
mod tests {
    #![allow(
        clippy::declare_interior_mutable_const,
        clippy::borrow_interior_mutable_const
    )]

    use super::AllowPrivateNetwork;
    use crate::cors::CorsLayer;

    use crate::test_helpers::Body;
    use http::{header::ORIGIN, request::Parts, HeaderName, HeaderValue, Request, Response};
    use tower::{BoxError, ServiceBuilder, ServiceExt};
    use tower_service::Service;

    const REQUEST_PRIVATE_NETWORK: HeaderName =
        HeaderName::from_static("access-control-request-private-network");

    const ALLOW_PRIVATE_NETWORK: HeaderName =
        HeaderName::from_static("access-control-allow-private-network");

    const TRUE: HeaderValue = HeaderValue::from_static("true");

    #[tokio::test]
    async fn cors_private_network_header_is_added_correctly() {
        let mut service = ServiceBuilder::new()
            .layer(CorsLayer::new().allow_private_network(true))
            .service_fn(echo);

        let req = Request::builder()
            .header(REQUEST_PRIVATE_NETWORK, TRUE)
            .body(Body::empty())
            .unwrap();
        let res = service.ready().await.unwrap().call(req).await.unwrap();

        assert_eq!(res.headers().get(ALLOW_PRIVATE_NETWORK).unwrap(), TRUE);

        let req = Request::builder().body(Body::empty()).unwrap();
        let res = service.ready().await.unwrap().call(req).await.unwrap();

        assert!(res.headers().get(ALLOW_PRIVATE_NETWORK).is_none());
    }

    #[tokio::test]
    async fn cors_private_network_header_is_added_correctly_with_predicate() {
        let allow_private_network =
            AllowPrivateNetwork::predicate(|origin: &HeaderValue, parts: &Parts| {
                parts.uri.path() == "/allow-private" && origin == "localhost"
            });
        let mut service = ServiceBuilder::new()
            .layer(CorsLayer::new().allow_private_network(allow_private_network))
            .service_fn(echo);

        let req = Request::builder()
            .header(ORIGIN, "localhost")
            .header(REQUEST_PRIVATE_NETWORK, TRUE)
            .uri("/allow-private")
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(res.headers().get(ALLOW_PRIVATE_NETWORK).unwrap(), TRUE);

        let req = Request::builder()
            .header(ORIGIN, "localhost")
            .header(REQUEST_PRIVATE_NETWORK, TRUE)
            .uri("/other")
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(req).await.unwrap();

        assert!(res.headers().get(ALLOW_PRIVATE_NETWORK).is_none());

        let req = Request::builder()
            .header(ORIGIN, "not-localhost")
            .header(REQUEST_PRIVATE_NETWORK, TRUE)
            .uri("/allow-private")
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(req).await.unwrap();

        assert!(res.headers().get(ALLOW_PRIVATE_NETWORK).is_none());
    }

    async fn echo(req: Request<Body>) -> Result<Response<Body>, BoxError> {
        Ok(Response::new(req.into_body()))
    }
}
