#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        set_header::{BoxedMakeHeaderValue, MakeHeaderValue as _},
        test_helpers::Body,
    };
    use http::{header, HeaderName, HeaderValue};
    use std::convert::Infallible;
    use tower::{service_fn, ServiceExt};

    #[tokio::test]
    async fn test_override_mode() {
        let svc = SetMultipleResponseHeader::overriding(
            service_fn(|_req: Request<Body>| async {
                let res = Response::builder()
                    .header(header::CONTENT_TYPE, "good-content")
                    .body(Body::empty())
                    .unwrap();
                Ok::<_, Infallible>(res)
            }),
            vec![(header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into()],
        );

        let res = svc.oneshot(Request::new(Body::empty())).await.unwrap();

        let mut values = res.headers().get_all(header::CONTENT_TYPE).iter();
        assert_eq!(values.next().unwrap(), "text/html");
        assert_eq!(values.next(), None);
    }

    #[tokio::test]
    async fn test_append_mode() {
        let svc = SetMultipleResponseHeader::appending(
            service_fn(|_req: Request<Body>| async {
                let res = Response::builder()
                    .header(header::CONTENT_TYPE, "good-content")
                    .body(Body::empty())
                    .unwrap();
                Ok::<_, Infallible>(res)
            }),
            vec![(header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into()],
        );

        let res = svc.oneshot(Request::new(Body::empty())).await.unwrap();

        let mut values = res.headers().get_all(header::CONTENT_TYPE).iter();
        assert_eq!(values.next().unwrap(), "good-content");
        assert_eq!(values.next().unwrap(), "text/html");
        assert_eq!(values.next(), None);
    }

    #[tokio::test]
    async fn test_skip_if_present_mode() {
        let svc = SetMultipleResponseHeader::if_not_present(
            service_fn(|_req: Request<Body>| async {
                let res = Response::builder()
                    .header(header::CONTENT_TYPE, "good-content")
                    .body(Body::empty())
                    .unwrap();
                Ok::<_, Infallible>(res)
            }),
            vec![(header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into()],
        );

        let res = svc.oneshot(Request::new(Body::empty())).await.unwrap();

        let mut values = res.headers().get_all(header::CONTENT_TYPE).iter();
        assert_eq!(values.next().unwrap(), "good-content");
        assert_eq!(values.next(), None);
    }

    #[tokio::test]
    async fn test_skip_if_present_mode_when_not_present() {
        let svc = SetMultipleResponseHeader::if_not_present(
            service_fn(|_req: Request<Body>| async {
                let res = Response::builder().body(Body::empty()).unwrap();
                Ok::<_, Infallible>(res)
            }),
            vec![(header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into()],
        );

        let res = svc.oneshot(Request::new(Body::empty())).await.unwrap();

        let mut values = res.headers().get_all(header::CONTENT_TYPE).iter();
        assert_eq!(values.next().unwrap(), "text/html");
        assert_eq!(values.next(), None);
    }

    #[test]
    fn test_tuple_metadata_impl() {
        let tuple: (HeaderName, HeaderValue) =
            (header::CONTENT_TYPE, HeaderValue::from_static("foo"));
        let meta: HeaderMetadata<HeaderValue> = tuple.into();
        assert_eq!(meta.header_name, header::CONTENT_TYPE);
        // Check that the header value is correct by making a header value from meta.make
        let mut make = meta.make.clone();
        assert_eq!(
            make.make_header_value(&HeaderValue::from_static("foo")),
            Some(HeaderValue::from_static("foo"))
        );
    }

    #[test]
    fn test_convert_to_header_config_struct_and_tuple() {
        let meta: HeaderMetadata<HeaderValue> = HeaderMetadata::<HeaderValue> {
            header_name: header::CONTENT_TYPE,
            make: BoxedMakeHeaderValue::new(HeaderValue::from_static("bar")),
        };
        let rh = meta.build_config(crate::set_header::InsertHeaderMode::Override);
        assert_eq!(rh.header_name, header::CONTENT_TYPE);
        let mut make = rh.make.clone();
        assert_eq!(
            make.make_header_value(&HeaderValue::from_static("bar")),
            Some(HeaderValue::from_static("bar"))
        );

        let tuple: (HeaderName, HeaderValue) =
            (header::CONTENT_TYPE, HeaderValue::from_static("baz"));
        let meta: HeaderMetadata<HeaderValue> = tuple.into();
        let rh2 = meta.build_config(crate::set_header::InsertHeaderMode::Override);
        assert_eq!(rh2.header_name, header::CONTENT_TYPE);
        let mut make2 = rh2.make.clone();
        assert_eq!(
            make2.make_header_value(&HeaderValue::from_static("baz")),
            Some(HeaderValue::from_static("baz"))
        );
    }

    #[test]
    fn test_debug_impls() {
        let meta: HeaderMetadata<HeaderValue> =
            (header::CONTENT_TYPE, HeaderValue::from_static("bar")).into();
        let rh = meta
            .clone()
            .build_config(crate::set_header::InsertHeaderMode::Override);
        let layer = SetMultipleResponseHeadersLayer::overriding(vec![meta]);
        let debug_str = format!("{:?}", layer);
        assert!(debug_str.contains("SetMultipleResponseHeadersLayer"));
        let debug_rh = format!("{:?}", rh);
        assert!(debug_rh.contains("HeaderInsertionConfig"));

        let svc = SetMultipleResponseHeader::overriding(
            tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, std::convert::Infallible>(Response::new(Body::empty()))
            }),
            vec![(header::CONTENT_TYPE, HeaderValue::from_static("foo")).into()]
                as Vec<HeaderMetadata<HeaderValue>>,
        );
        let debug_svc = format!("{:?}", svc);
        assert!(debug_svc.contains("SetMultipleResponseHeader"));
    }

    #[tokio::test]
    async fn test_layer_construction_and_multiple_headers() {
        // Multiple different headers in the same vec
        let svc = tower::ServiceBuilder::new()
            .layer(SetMultipleResponseHeadersLayer::overriding(vec![
                (header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into(),
                (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")).into(),
            ]))
            .service(service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(Response::new(Body::empty()))
            }));

        let res = svc.oneshot(Request::new(Body::empty())).await.unwrap();
        assert_eq!(res.headers()["content-type"], "text/html");
        assert_eq!(res.headers()["cache-control"], "no-cache");
    }

    #[tokio::test]
    async fn test_layer_with_empty_vec() {
        let svc = tower::ServiceBuilder::new()
            .layer(SetMultipleResponseHeadersLayer::<Response<Body>>::overriding(vec![]))
            .service(service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(Response::new(Body::empty()))
            }));

        let res = svc.oneshot(Request::new(Body::empty())).await.unwrap();
        // No headers should be set
        assert_eq!(res.headers().len(), 0);
    }

    #[tokio::test]
    async fn test_layer_with_static_and_closure_headers_fixed() {
        // Wrap the static value
        let static_meta = (header::CONTENT_TYPE, HeaderValue::from_static("text/html")).into();

        // Wrap the closure
        let closure_meta = (header::X_FRAME_OPTIONS, |_res: &Response<Body>| {
            Some(HeaderValue::from_static("DENY"))
        })
            .into();

        let svc = tower::ServiceBuilder::new()
            .layer(SetMultipleResponseHeadersLayer::overriding(vec![
                static_meta,
                closure_meta,
            ]))
            .service(service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(Response::new(Body::empty()))
            }));

        let res = svc.oneshot(Request::new(Body::empty())).await.unwrap();
        assert_eq!(res.headers()["content-type"], "text/html");
        assert_eq!(res.headers()["x-frame-options"], "DENY");
    }

    #[test]
    fn test_debug_layer_and_service() {
        let meta: HeaderMetadata<HeaderValue> =
            (header::CONTENT_TYPE, HeaderValue::from_static("foo")).into();
        let layer = SetMultipleResponseHeadersLayer::overriding(vec![meta]);
        let debug_str = format!("{:?}", layer);
        assert!(debug_str.contains("SetMultipleResponseHeadersLayer"));
    }
}
