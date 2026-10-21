#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::ServerErrorsFailureClass;
    use crate::test_helpers::Body;
    use bytes::Bytes;
    use http::{HeaderMap, Request, Response};
    use once_cell::sync::Lazy;
    use std::{
        sync::atomic::{AtomicU32, Ordering},
        time::Duration,
    };
    use service_stack::{BoxError, Service, ServiceBuilder, ServiceExt};
    use tracing::Span;

    #[tokio::test]
    async fn unary_request() {
        static ON_REQUEST_COUNT: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_RESPONSE_COUNT: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_BODY_CHUNK_COUNT: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_EOS: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_FAILURE: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));

        let trace_layer = TraceLayer::new_for_http()
            .make_span_with(|_req: &Request<Body>| {
                tracing::info_span!("test-span", foo = tracing::field::Empty)
            })
            .on_request(|_req: &Request<Body>, span: &Span| {
                span.record("foo", 42);
                ON_REQUEST_COUNT.fetch_add(1, Ordering::SeqCst);
            })
            .on_response(|_res: &Response<Body>, _latency: Duration, _span: &Span| {
                ON_RESPONSE_COUNT.fetch_add(1, Ordering::SeqCst);
            })
            .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {
                ON_BODY_CHUNK_COUNT.fetch_add(1, Ordering::SeqCst);
            })
            .on_eos(
                |_trailers: Option<&HeaderMap>, _latency: Duration, _span: &Span| {
                    ON_EOS.fetch_add(1, Ordering::SeqCst);
                },
            )
            .on_failure(
                |_class: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                    ON_FAILURE.fetch_add(1, Ordering::SeqCst);
                },
            );

        let mut svc = ServiceBuilder::new().layer(trace_layer).service_fn(echo);

        let res = svc
            .ready()
            .await
            .unwrap()
            .call(Request::new(Body::from("foobar")))
            .await
            .unwrap();

        assert_eq!(1, ON_REQUEST_COUNT.load(Ordering::SeqCst), "request");
        assert_eq!(1, ON_RESPONSE_COUNT.load(Ordering::SeqCst), "request");
        assert_eq!(0, ON_BODY_CHUNK_COUNT.load(Ordering::SeqCst), "body chunk");
        assert_eq!(0, ON_EOS.load(Ordering::SeqCst), "eos");
        assert_eq!(0, ON_FAILURE.load(Ordering::SeqCst), "failure");

        crate::test_helpers::to_bytes(res.into_body())
            .await
            .unwrap();
        assert_eq!(1, ON_BODY_CHUNK_COUNT.load(Ordering::SeqCst), "body chunk");
        assert_eq!(1, ON_EOS.load(Ordering::SeqCst), "eos");
        assert_eq!(0, ON_FAILURE.load(Ordering::SeqCst), "failure");
    }

    #[tokio::test]
    async fn streaming_response() {
        static ON_REQUEST_COUNT: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_RESPONSE_COUNT: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_BODY_CHUNK_COUNT: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_EOS: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_FAILURE: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));

        let trace_layer = TraceLayer::new_for_http()
            .on_request(|_req: &Request<Body>, _span: &Span| {
                ON_REQUEST_COUNT.fetch_add(1, Ordering::SeqCst);
            })
            .on_response(|_res: &Response<Body>, _latency: Duration, _span: &Span| {
                ON_RESPONSE_COUNT.fetch_add(1, Ordering::SeqCst);
            })
            .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {
                ON_BODY_CHUNK_COUNT.fetch_add(1, Ordering::SeqCst);
            })
            .on_eos(
                |_trailers: Option<&HeaderMap>, _latency: Duration, _span: &Span| {
                    ON_EOS.fetch_add(1, Ordering::SeqCst);
                },
            )
            .on_failure(
                |_class: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                    ON_FAILURE.fetch_add(1, Ordering::SeqCst);
                },
            );

        let mut svc = ServiceBuilder::new()
            .layer(trace_layer)
            .service_fn(streaming_body);

        let res = svc
            .ready()
            .await
            .unwrap()
            .call(Request::new(Body::empty()))
            .await
            .unwrap();

        assert_eq!(1, ON_REQUEST_COUNT.load(Ordering::SeqCst), "request");
        assert_eq!(1, ON_RESPONSE_COUNT.load(Ordering::SeqCst), "request");
        assert_eq!(0, ON_BODY_CHUNK_COUNT.load(Ordering::SeqCst), "body chunk");
        assert_eq!(0, ON_EOS.load(Ordering::SeqCst), "eos");
        assert_eq!(0, ON_FAILURE.load(Ordering::SeqCst), "failure");

        crate::test_helpers::to_bytes(res.into_body())
            .await
            .unwrap();
        assert_eq!(3, ON_BODY_CHUNK_COUNT.load(Ordering::SeqCst), "body chunk");
        assert_eq!(1, ON_EOS.load(Ordering::SeqCst), "eos");
        assert_eq!(0, ON_FAILURE.load(Ordering::SeqCst), "failure");
    }

    #[tokio::test]
    async fn classify_eos_on_trailers_success() {
        static ON_EOS: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_FAILURE: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));

        let trace_layer = TraceLayer::new(TestClassify::new(false))
            .on_eos(
                |_trailers: Option<&HeaderMap>, _latency: Duration, _span: &Span| {
                    ON_EOS.fetch_add(1, Ordering::SeqCst);
                },
            )
            .on_failure(|_class: &'static str, _latency: Duration, _span: &Span| {
                ON_FAILURE.fetch_add(1, Ordering::SeqCst);
            });

        let mut svc = ServiceBuilder::new()
            .layer(trace_layer)
            .service_fn(body_with_trailers);

        let res = svc
            .ready()
            .await
            .unwrap()
            .call(Request::new(Body::empty()))
            .await
            .unwrap();

        crate::test_helpers::to_bytes(res.into_body())
            .await
            .unwrap();
        assert_eq!(1, ON_EOS.load(Ordering::SeqCst), "eos");
        assert_eq!(0, ON_FAILURE.load(Ordering::SeqCst), "failure");
    }

    #[tokio::test]
    async fn classify_eos_on_trailers_failure() {
        static ON_EOS: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_FAILURE: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));

        let trace_layer = TraceLayer::new(TestClassify::new(true))
            .on_eos(
                |_trailers: Option<&HeaderMap>, _latency: Duration, _span: &Span| {
                    ON_EOS.fetch_add(1, Ordering::SeqCst);
                },
            )
            .on_failure(|_class: &'static str, _latency: Duration, _span: &Span| {
                ON_FAILURE.fetch_add(1, Ordering::SeqCst);
            });

        let mut svc = ServiceBuilder::new()
            .layer(trace_layer)
            .service_fn(body_with_trailers);

        let res = svc
            .ready()
            .await
            .unwrap()
            .call(Request::new(Body::empty()))
            .await
            .unwrap();

        crate::test_helpers::to_bytes(res.into_body())
            .await
            .unwrap();
        assert_eq!(1, ON_EOS.load(Ordering::SeqCst), "eos");
        assert_eq!(1, ON_FAILURE.load(Ordering::SeqCst), "failure");
    }

    #[tokio::test]
    async fn classify_eos_on_empty_stream() {
        static ON_EOS: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));
        static ON_FAILURE: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));

        let trace_layer = TraceLayer::new(TestClassify::new(true))
            .on_eos(
                |_trailers: Option<&HeaderMap>, _latency: Duration, _span: &Span| {
                    ON_EOS.fetch_add(1, Ordering::SeqCst);
                },
            )
            .on_failure(|_class: &'static str, _latency: Duration, _span: &Span| {
                ON_FAILURE.fetch_add(1, Ordering::SeqCst);
            });

        let mut svc = ServiceBuilder::new()
            .layer(trace_layer)
            .service_fn(streaming_body);

        let res = svc
            .ready()
            .await
            .unwrap()
            .call(Request::new(Body::empty()))
            .await
            .unwrap();

        crate::test_helpers::to_bytes(res.into_body())
            .await
            .unwrap();
        assert_eq!(1, ON_EOS.load(Ordering::SeqCst), "eos");
        assert_eq!(1, ON_FAILURE.load(Ordering::SeqCst), "failure");
    }

    async fn echo(req: Request<Body>) -> Result<Response<Body>, BoxError> {
        Ok(Response::new(req.into_body()))
    }

    async fn streaming_body(_req: Request<Body>) -> Result<Response<Body>, BoxError> {
        use futures::stream::iter;

        let stream = iter(vec![
            Ok::<_, BoxError>(Bytes::from("one")),
            Ok::<_, BoxError>(Bytes::from("two")),
            Ok::<_, BoxError>(Bytes::from("three")),
        ]);

        let body = Body::from_stream(stream);

        Ok(Response::new(body))
    }

    async fn body_with_trailers(_req: Request<Body>) -> Result<Response<Body>, BoxError> {
        let mut trailers = HeaderMap::new();
        trailers.insert("x-test-trailer", "value".parse().unwrap());
        let body = Body::new(Body::from(Bytes::from("data")).with_trailers(trailers));
        Ok(Response::new(body))
    }

    #[derive(Clone)]
    struct TestClassify {
        reject: bool,
    }

    impl TestClassify {
        fn new(reject: bool) -> Self {
            Self { reject }
        }
    }

    impl crate::classify::MakeClassifier for TestClassify {
        type FailureClass = &'static str;
        type ClassifyEos = TestClassifyEos;
        type Classifier = TestClassifyResponse;

        fn make_classifier<B>(&self, _req: &Request<B>) -> Self::Classifier {
            TestClassifyResponse {
                reject: self.reject,
            }
        }
    }

    #[derive(Clone)]
    struct TestClassifyResponse {
        reject: bool,
    }

    impl crate::classify::ClassifyResponse for TestClassifyResponse {
        type FailureClass = &'static str;
        type ClassifyEos = TestClassifyEos;

        fn classify_response<B>(
            self,
            _res: &Response<B>,
        ) -> crate::classify::ClassifiedResponse<Self::FailureClass, Self::ClassifyEos> {
            crate::classify::ClassifiedResponse::RequiresEos(TestClassifyEos {
                reject: self.reject,
            })
        }

        fn classify_error<E>(self, _error: &E) -> Self::FailureClass
        where
            E: std::fmt::Display + 'static,
        {
            "error"
        }
    }

    #[derive(Clone)]
    struct TestClassifyEos {
        reject: bool,
    }

    impl crate::classify::ClassifyEos for TestClassifyEos {
        type FailureClass = &'static str;

        fn classify_eos(self, _trailers: Option<&HeaderMap>) -> Result<(), Self::FailureClass> {
            if self.reject {
                Err("classified as failure")
            } else {
                Ok(())
            }
        }

        fn classify_error<E>(self, _error: &E) -> Self::FailureClass
        where
            E: std::fmt::Display + 'static,
        {
            "error"
        }
    }
}
