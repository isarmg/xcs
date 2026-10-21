#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::Body;
    use http::{header, HeaderValue, Request, Response};
    use std::convert::Infallible;
    use tower::{service_fn, ServiceExt};

    #[tokio::test]
    async fn test_override_mode() {
        let svc = SetMultipleRequestHeader::overriding(
            service_fn(|req: Request<Body>| async move {
                assert_eq!(req.headers()["content-type"], "text/html");
                Ok::<_, Infallible>(Response::new(Body::empty()))
            }),
            vec![(header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into()],
        );

        let mut req = Request::new(Body::empty());

        // Add an initial CONTENT_TYPE header to the request
        req.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("good-content"),
        );

        let _ = svc.oneshot(req).await.unwrap();
    }

    #[tokio::test]
    async fn test_append_mode() {
        let svc = SetMultipleRequestHeader::appending(
            service_fn(|req: Request<Body>| async move {
                let mut values = req.headers().get_all("content-type").iter();
                assert_eq!(values.next().unwrap(), "good-content");
                assert_eq!(values.next().unwrap(), "text/html");
                assert_eq!(values.next(), None);

                Ok::<_, Infallible>(Response::new(Body::empty()))
            }),
            vec![(header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into()],
        );

        // Add an initial CONTENT_TYPE header to the request
        let mut req = Request::new(Body::empty());
        req.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("good-content"),
        );

        _ = svc.oneshot(req).await.unwrap();
    }

    #[tokio::test]
    async fn test_skip_if_present_mode() {
        let svc = SetMultipleRequestHeader::if_not_present(
            service_fn(|req: Request<Body>| async move {
                let mut values = req.headers().get_all("content-type").iter();
                assert_eq!(values.next().unwrap(), "good-content");
                assert_eq!(values.next(), None);

                Ok::<_, Infallible>(Response::new(Body::empty()))
            }),
            vec![(header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into()],
        );

        // Add an initial CONTENT_TYPE header to the request
        let mut req = Request::new(Body::empty());
        req.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("good-content"),
        );

        let _ = svc.oneshot(req).await.unwrap();
    }

    #[tokio::test]
    async fn test_skip_if_present_mode_when_not_present() {
        let svc = SetMultipleRequestHeader::if_not_present(
            service_fn(|req: Request<Body>| async move {
                let mut values = req.headers().get_all("content-type").iter();
                assert_eq!(values.next().unwrap(), "text/html");
                assert_eq!(values.next(), None);
                Ok::<_, Infallible>(Response::new(Body::empty()))
            }),
            vec![(header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into()],
        );

        // No header to the request
        let req = Request::new(Body::empty());

        _ = svc.oneshot(req).await.unwrap();
    }

    #[test]
    fn test_debug_impls() {
        let meta: HeaderMetadata<HeaderValue> =
            (header::CONTENT_TYPE, HeaderValue::from_static("bar")).into();
        let rh = meta
            .clone()
            .build_config(crate::set_header::InsertHeaderMode::Override);
        let layer = SetMultipleRequestHeadersLayer::overriding(vec![meta]);
        let debug_str = format!("{:?}", layer);
        assert!(debug_str.contains("SetMultipleRequestHeadersLayer"));
        let debug_rh = format!("{:?}", rh);
        assert!(debug_rh.contains("HeaderInsertionConfig"));

        let svc = SetMultipleRequestHeader::overriding(
            tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, std::convert::Infallible>(Response::new(Body::empty()))
            }),
            vec![(header::CONTENT_TYPE, HeaderValue::from_static("foo")).into()]
                as Vec<HeaderMetadata<HeaderValue>>,
        );
        let debug_svc = format!("{:?}", svc);
        assert!(debug_svc.contains("SetMultipleRequestHeader"));
    }

    #[tokio::test]
    async fn test_layer_construction_and_multiple_headers() {
        // Multiple different headers in the same vec
        let svc = tower::ServiceBuilder::new()
            .layer(SetMultipleRequestHeadersLayer::overriding(vec![
                (header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into(),
                (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")).into(),
            ]))
            .service(service_fn(|req: Request<Body>| async move {
                assert_eq!(req.headers()["content-type"], "text/html");
                assert_eq!(req.headers()["cache-control"], "no-cache");

                Ok::<_, Infallible>(Response::new(Body::empty()))
            }));

        _ = svc.oneshot(Request::new(Body::empty())).await.unwrap();
    }

    #[tokio::test]
    async fn test_layer_with_empty_vec() {
        let header_metadatas: Vec<HeaderMetadata<Request<Body>>> = vec![];
        let svc = tower::ServiceBuilder::new()
            .layer(SetMultipleRequestHeadersLayer::<Request<Body>>::overriding(
                header_metadatas,
            ))
            .service(service_fn(|req: Request<Body>| async move {
                assert_eq!(req.headers().len(), 0);
                Ok::<_, Infallible>(Response::new(Body::empty()))
            }));

        _ = svc.oneshot(Request::new(Body::empty())).await.unwrap();
    }

    #[tokio::test]
    async fn test_layer_with_static_and_closure_headers_fixed() {
        // Wrap the static value
        let static_meta: HeaderMetadata<Request<Body>> =
            (header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into();

        // Wrap the closure
        let closure_meta: HeaderMetadata<Request<Body>> =
            (header::X_FRAME_OPTIONS, |_req: &Request<Body>| {
                Some(HeaderValue::from_static("DENY"))
            })
                .into();

        let svc = tower::ServiceBuilder::new()
            .layer(SetMultipleRequestHeadersLayer::overriding(vec![
                static_meta,
                closure_meta,
            ]))
            .service(service_fn(|req: Request<Body>| async move {
                assert_eq!(req.headers()["content-type"], "text/html");
                assert_eq!(req.headers()["x-frame-options"], "DENY");

                Ok::<_, Infallible>(Response::new(Body::empty()))
            }));

        _ = svc.oneshot(Request::new(Body::empty())).await.unwrap();
    }

    #[test]
    fn test_debug_layer_and_service() {
        let meta: HeaderMetadata<HeaderValue> =
            (header::CONTENT_TYPE, HeaderValue::from_static("foo")).into();
        let layer = SetMultipleRequestHeadersLayer::overriding(vec![meta]);
        let debug_str = format!("{:?}", layer);
        assert!(debug_str.contains("SetMultipleRequestHeadersLayer"));
    }
}
