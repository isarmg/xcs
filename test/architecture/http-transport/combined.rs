// Include this module in the combined HTTP architecture probe.
// Required existing capabilities: service-stack/util; HTTP transport client,
// server, http1, service-adapter and tokio; Tokio macros, rt, io-util and time.
// http-routing can have default features disabled: this test does not use serve.
mod combined_http_stack {
    use http_body::{BodyExt, Empty, Full};
    use http_routing::{extract::Path, routing::post, Router};
    use http_transport::{
        adapters::{rt::TokioIo, service::TowerToHyperService},
        body::Bytes,
        Request, Response, StatusCode,
    };
    use service_stack::ServiceBuilder;
    use std::{
        convert::Infallible,
        future::{ready, Future, Ready},
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        task::{Context, Poll, Wake, Waker},
        time::Duration,
    };

    #[tokio::test]
    async fn router_and_middleware_cross_the_actual_http1_transport() {
        let app: Router = Router::new().route(
            "/items/{id}",
            post(|Path(id): Path<String>, body: String| async move {
                (StatusCode::CREATED, format!("{id}:{body}"))
            }),
        );
        let middleware_calls = Arc::new(AtomicUsize::new(0));
        let observed_calls = Arc::clone(&middleware_calls);
        let service = ServiceBuilder::new()
            .map_response(move |mut response: http_routing::response::Response| {
                observed_calls.fetch_add(1, Ordering::SeqCst);
                response.headers_mut().insert(
                    "x-service-stack",
                    http_transport::http::HeaderValue::from_static("applied"),
                );
                response
            })
            .service(app);
        let adapted = TowerToHyperService::new(service);
        let (client_io, server_io) = tokio::io::duplex(4096);
        let server_task = tokio::spawn(async move {
            http_transport::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(server_io), adapted)
                .await
        });
        let (mut sender, connection) =
            http_transport::client::conn::http1::handshake(TokioIo::new(client_io))
                .await
                .expect("HTTP/1 client handshake");
        let client_task = tokio::spawn(connection);

        let response = tokio::time::timeout(
            Duration::from_secs(5),
            sender.send_request(
                Request::builder()
                    .method("POST")
                    .uri("/items/42")
                    .header("host", "local.test")
                    .body(Full::new(Bytes::from_static(b"payload")))
                    .expect("valid request"),
            ),
        )
        .await
        .expect("HTTP/1 response must not hang")
        .expect("HTTP/1 response");
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()["x-service-stack"], "applied");
        let body = tokio::time::timeout(Duration::from_secs(5), response.into_body().collect())
            .await
            .expect("response body must not hang")
            .expect("response body")
            .to_bytes();
        assert_eq!(body, "42:payload");
        assert_eq!(middleware_calls.load(Ordering::SeqCst), 1);

        drop(sender);
        tokio::time::timeout(Duration::from_secs(5), client_task)
            .await
            .expect("client connection must shut down")
            .expect("client task must not panic")
            .expect("client connection must complete cleanly");
        tokio::time::timeout(Duration::from_secs(5), server_task)
            .await
            .expect("server connection must shut down")
            .expect("server task must not panic")
            .expect("server connection must complete cleanly");
    }

    #[derive(Default)]
    struct ReadinessCounters {
        polls: AtomicUsize,
        calls: AtomicUsize,
        wakes: AtomicUsize,
    }

    #[derive(Clone)]
    struct PendingOnceService {
        counters: Arc<ReadinessCounters>,
        yielded: bool,
        ready: bool,
    }

    impl service_stack::Service<Request<Empty<Bytes>>> for PendingOnceService {
        type Response = Response<Empty<Bytes>>;
        type Error = Infallible;
        type Future = Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.counters.polls.fetch_add(1, Ordering::SeqCst);
            if !self.yielded {
                self.yielded = true;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            self.ready = true;
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _: Request<Empty<Bytes>>) -> Self::Future {
            assert!(self.ready, "adapter called a service that was not ready");
            self.ready = false;
            self.counters.calls.fetch_add(1, Ordering::SeqCst);
            ready(Ok(Response::new(Empty::new())))
        }
    }

    struct CountWake(Arc<ReadinessCounters>);

    impl Wake for CountWake {
        fn wake(self: Arc<Self>) {
            self.0.wakes.fetch_add(1, Ordering::SeqCst);
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.wakes.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn service_adapter_waits_for_readiness_before_calling() {
        let counters = Arc::new(ReadinessCounters::default());
        let adapted = TowerToHyperService::new(PendingOnceService {
            counters: Arc::clone(&counters),
            yielded: false,
            ready: false,
        });
        let mut response = Box::pin(http_transport::service::Service::call(
            &adapted,
            Request::new(Empty::<Bytes>::new()),
        ));
        assert_eq!(counters.polls.load(Ordering::SeqCst), 0);
        assert_eq!(counters.calls.load(Ordering::SeqCst), 0);
        let waker = Waker::from(Arc::new(CountWake(Arc::clone(&counters))));
        let mut cx = Context::from_waker(&waker);

        assert!(response.as_mut().poll(&mut cx).is_pending());
        assert_eq!(counters.polls.load(Ordering::SeqCst), 1);
        assert_eq!(counters.calls.load(Ordering::SeqCst), 0);
        assert_eq!(counters.wakes.load(Ordering::SeqCst), 1);

        // The first readiness poll explicitly woke us. Poll again without a
        // runtime, timer, sleep or scheduling assumption.
        let Poll::Ready(result) = response.as_mut().poll(&mut cx) else {
            panic!("the explicitly woken service should now be ready");
        };
        assert_eq!(result.expect("infallible service").status(), StatusCode::OK);
        assert_eq!(counters.polls.load(Ordering::SeqCst), 2);
        assert_eq!(counters.calls.load(Ordering::SeqCst), 1);
        assert_eq!(counters.wakes.load(Ordering::SeqCst), 1);
    }
}
