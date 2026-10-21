#[cfg(test)]
mod tests {
    use crate::test_helpers::Body;
    use crate::ServiceBuilderExt as _;
    use http::Response;
    use std::sync::atomic::AtomicUsize;
    use std::{
        convert::Infallible,
        sync::{atomic::Ordering, Arc},
    };
    use tower::{ServiceBuilder, ServiceExt};

    #[allow(unused_imports)]
    use super::*;

    #[tokio::test]
    async fn basic() {
        let svc = ServiceBuilder::new()
            .set_x_request_id(Counter::default())
            .propagate_x_request_id()
            .service_fn(handler);

        // header on response
        let req = Request::builder().body(Body::empty()).unwrap();
        let res = svc.clone().oneshot(req).await.unwrap();
        assert_eq!(res.headers()["x-request-id"], "0");

        let req = Request::builder().body(Body::empty()).unwrap();
        let res = svc.clone().oneshot(req).await.unwrap();
        assert_eq!(res.headers()["x-request-id"], "1");

        // doesn't override if header is already there
        let req = Request::builder()
            .header("x-request-id", "foo")
            .body(Body::empty())
            .unwrap();
        let res = svc.clone().oneshot(req).await.unwrap();
        assert_eq!(res.headers()["x-request-id"], "foo");

        // extension propagated
        let req = Request::builder().body(Body::empty()).unwrap();
        let res = svc.clone().oneshot(req).await.unwrap();
        assert_eq!(res.extensions().get::<RequestId>().unwrap().0, "2");
    }

    #[tokio::test]
    async fn other_middleware_setting_request_id() {
        let svc = ServiceBuilder::new()
            .override_request_header(
                HeaderName::from_static("x-request-id"),
                HeaderValue::from_str("foo").unwrap(),
            )
            .set_x_request_id(Counter::default())
            .map_request(|request: Request<_>| {
                // `set_x_request_id` should set the extension if its missing
                assert_eq!(request.extensions().get::<RequestId>().unwrap().0, "foo");
                request
            })
            .propagate_x_request_id()
            .service_fn(handler);

        let req = Request::builder()
            .header(
                "x-request-id",
                "this-will-be-overriden-by-override_request_header-middleware",
            )
            .body(Body::empty())
            .unwrap();
        let res = svc.clone().oneshot(req).await.unwrap();
        assert_eq!(res.headers()["x-request-id"], "foo");
        assert_eq!(res.extensions().get::<RequestId>().unwrap().0, "foo");
    }

    #[tokio::test]
    async fn other_middleware_setting_request_id_on_response() {
        let svc = ServiceBuilder::new()
            .set_x_request_id(Counter::default())
            .propagate_x_request_id()
            .override_response_header(
                HeaderName::from_static("x-request-id"),
                HeaderValue::from_str("foo").unwrap(),
            )
            .service_fn(handler);

        let req = Request::builder()
            .header("x-request-id", "foo")
            .body(Body::empty())
            .unwrap();
        let res = svc.clone().oneshot(req).await.unwrap();
        assert_eq!(res.headers()["x-request-id"], "foo");
        assert_eq!(res.extensions().get::<RequestId>().unwrap().0, "foo");
    }

    #[derive(Clone, Default)]
    struct Counter(Arc<AtomicUsize>);

    impl MakeRequestId for Counter {
        fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
            let id =
                HeaderValue::from_str(&self.0.fetch_add(1, Ordering::SeqCst).to_string()).unwrap();
            Some(RequestId::new(id))
        }
    }

    async fn handler(_: Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(Response::new(Body::empty()))
    }

    #[tokio::test]
    async fn uuid() {
        let svc = ServiceBuilder::new()
            .set_x_request_id(MakeRequestUuid)
            .propagate_x_request_id()
            .service_fn(handler);

        // header on response
        let req = Request::builder().body(Body::empty()).unwrap();
        let mut res = svc.clone().oneshot(req).await.unwrap();
        let id = res.headers_mut().remove("x-request-id").unwrap();
        id.to_str().unwrap().parse::<Uuid>().unwrap();
    }
}
