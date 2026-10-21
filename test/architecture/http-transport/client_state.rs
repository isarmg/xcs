// HTTP/1-only legacy client boundaries, tested without network sockets.
mod client_state {
    use http_body::{BodyExt, Empty, Full};
    use http_transport::{
        adapters::{
            client::legacy::{
                connect::{Connected, Connection},
                Client,
            },
            rt::{TokioExecutor, TokioIo},
        },
        body::{Bytes, Incoming},
        rt::{Read, ReadBufCursor, Write},
        service::service_fn,
        Request, Response, Uri, Version,
    };
    use std::{
        convert::Infallible,
        future::Future,
        io,
        pin::Pin,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
        task::{Context, Poll},
        time::Duration,
    };
    use tokio::io::{AsyncReadExt, DuplexStream};

    struct Transport {
        io: TokioIo<DuplexStream>,
        h2: bool,
    }
    impl Connection for Transport {
        fn connected(&self) -> Connected {
            if self.h2 {
                Connected::new().negotiated_h2()
            } else {
                Connected::new()
            }
        }
    }
    impl Read for Transport {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: ReadBufCursor<'_>,
        ) -> Poll<io::Result<()>> {
            Pin::new(&mut self.io).poll_read(cx, buf)
        }
    }
    impl Write for Transport {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut self.io).poll_write(cx, buf)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.io).poll_flush(cx)
        }
        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.io).poll_shutdown(cx)
        }
    }
    async fn bounded<T>(future: impl Future<Output = T>) -> T {
        tokio::time::timeout(Duration::from_secs(5), future)
            .await
            .expect("client operation must not hang")
    }

    #[tokio::test]
    async fn unsupported_request_versions_do_not_connect() {
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let connector = service_stack::service_fn(move |_: Uri| {
            observed.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Err::<Transport, _>(io::Error::other(
                "connector must not be used",
            )))
        });
        let client: Client<_, Empty<Bytes>> =
            Client::builder(TokioExecutor::new()).build(connector);
        for version in [Version::HTTP_09, Version::HTTP_2, Version::HTTP_3] {
            let req = Request::builder()
                .uri("http://local.test/")
                .version(version)
                .body(Empty::new())
                .unwrap();
            let error = bounded(client.request(req)).await.unwrap_err();
            assert!(
                !error.is_connect(),
                "version rejection must precede connecting: {error:?}"
            );
            assert!(format!("{error:?}").contains("UserUnsupportedVersion"));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn h2_alpn_returns_error_without_writing_or_poisoning_pool() {
        let (first, mut peer) = tokio::io::duplex(4096);
        let (second, server) = tokio::io::duplex(4096);
        let connections = Arc::new(Mutex::new(std::collections::VecDeque::from([
            Transport {
                io: TokioIo::new(first),
                h2: true,
            },
            Transport {
                io: TokioIo::new(second),
                h2: false,
            },
        ])));
        let connector = service_stack::service_fn(move |_: Uri| {
            std::future::ready(
                connections
                    .lock()
                    .unwrap()
                    .pop_front()
                    .ok_or_else(|| io::Error::other("unexpected connection")),
            )
        });
        let client: Client<_, Empty<Bytes>> =
            Client::builder(TokioExecutor::new()).build(connector);
        let error = bounded(client.get("http://local.test/".parse().unwrap()))
            .await
            .unwrap_err();
        assert!(error.is_connect());
        assert!(std::error::Error::source(&error)
            .unwrap()
            .to_string()
            .contains("HTTP/2 ALPN"));
        let mut wire = Vec::new();
        bounded(peer.read_to_end(&mut wire)).await.unwrap();
        assert!(
            wire.is_empty(),
            "must never send HTTP/1 bytes on an H2-negotiated stream"
        );
        let server_task = tokio::spawn(async move {
            http_transport::server::conn::http1::Builder::new()
                .serve_connection(
                    TokioIo::new(server),
                    service_fn(|_: Request<Incoming>| async {
                        Ok::<_, Infallible>(Response::new(Full::new(Bytes::from_static(
                            b"recovered",
                        ))))
                    }),
                )
                .await
        });
        let response = bounded(client.get("http://local.test/".parse().unwrap()))
            .await
            .unwrap();
        assert_eq!(
            bounded(response.into_body().collect())
                .await
                .unwrap()
                .to_bytes(),
            "recovered"
        );
        drop(client);
        bounded(server_task).await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn http1_requests_reuse_connection_after_body_and_dispatcher_ready() {
        let (io, server) = tokio::io::duplex(64);
        let transport = Arc::new(Mutex::new(Some(Transport {
            io: TokioIo::new(io),
            h2: false,
        })));
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let connector = service_stack::service_fn(move |_: Uri| {
            observed.fetch_add(1, Ordering::SeqCst);
            std::future::ready(
                transport
                    .lock()
                    .unwrap()
                    .take()
                    .ok_or_else(|| io::Error::other("must reuse original connection")),
            )
        });
        let server_task = tokio::spawn(async move {
            http_transport::server::conn::http1::Builder::new()
                .serve_connection(
                    TokioIo::new(server),
                    service_fn(|request: Request<Incoming>| async {
                        assert_eq!(request.headers()["host"], "local.test");
                        assert_eq!(request.uri(), "/payload?part=1");
                        let bytes = request.into_body().collect().await.unwrap().to_bytes();
                        Ok::<_, Infallible>(Response::new(Full::new(bytes)))
                    }),
                )
                .await
        });
        let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build(connector);
        for index in 0..3 {
            let payload = format!("{index}:{}", "body".repeat(512));
            let req = Request::post("http://local.test/payload?part=1")
                .body(Full::new(Bytes::from(payload.clone())))
                .unwrap();
            let response = bounded(client.request(req)).await.unwrap();
            assert_eq!(
                bounded(response.into_body().collect())
                    .await
                    .unwrap()
                    .to_bytes(),
                payload
            );
            // The dispatcher observes completion and makes its sender ready.
            tokio::task::yield_now().await;
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        drop(client);
        bounded(server_task).await.unwrap().unwrap();
    }

    #[test]
    fn auto_builder_advertises_only_http1() {
        let auto = http_transport::adapters::server::conn::auto::Builder::new();
        assert!(auto.is_http1_available());
        assert!(!auto.is_http2_available());
        let explicit = auto.http1_only();
        assert!(explicit.is_http1_available());
        assert!(!explicit.is_http2_available());
    }

    #[tokio::test]
    async fn auto_server_rejects_h2_preface_before_calling_service() {
        use tokio::io::AsyncWriteExt;
        for upgrades in [false, true] {
            let (mut peer, server) = tokio::io::duplex(64);
            let calls = Arc::new(AtomicUsize::new(0));
            let observed = calls.clone();
            let server_task = tokio::spawn(async move {
                let builder = http_transport::adapters::server::conn::auto::Builder::new();
                let service = service_fn(move |_: Request<Incoming>| {
                    observed.fetch_add(1, Ordering::SeqCst);
                    std::future::ready(Ok::<_, Infallible>(Response::new(Empty::<Bytes>::new())))
                });
                if upgrades {
                    builder
                        .serve_connection_with_upgrades(TokioIo::new(server), service)
                        .await
                } else {
                    builder
                        .serve_connection(TokioIo::new(server), service)
                        .await
                }
            });
            bounded(peer.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"))
                .await
                .unwrap();
            let error = bounded(server_task).await.unwrap().unwrap_err();
            assert_eq!(error.to_string(), "HTTP/2 is not supported");
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }
    }

    #[tokio::test]
    async fn executor_free_owned_connections_serve_http1() {
        use tokio::io::AsyncWriteExt;
        for upgrades in [false, true] {
            let (mut peer, server) = tokio::io::duplex(4096);
            let task = {
                let mut builder = http_transport::adapters::server::conn::auto::Builder::default();
                builder.http1().keep_alive(false).title_case_headers(true);
                let service = service_fn(|_: Request<Incoming>| async {
                    Ok::<_, Infallible>(Response::new(Full::new(Bytes::from_static(b"owned"))))
                });
                // Both futures must outlive the builder and remain Send for spawning.
                if upgrades {
                    tokio::spawn(
                        builder
                            .serve_connection_with_upgrades(TokioIo::new(server), service)
                            .into_owned(),
                    )
                } else {
                    tokio::spawn(
                        builder
                            .serve_connection(TokioIo::new(server), service)
                            .into_owned(),
                    )
                }
            };
            bounded(peer.write_all(b"GET / HTTP/1.1\r\nHost: local.test\r\n\r\n"))
                .await
                .unwrap();
            let mut wire = Vec::new();
            bounded(peer.read_to_end(&mut wire)).await.unwrap();
            bounded(task).await.unwrap().unwrap();
            assert!(wire.starts_with(b"HTTP/1.1 200"));
            assert!(wire.ends_with(b"owned"));
            assert!(String::from_utf8(wire)
                .unwrap()
                .contains("\r\nContent-Length: 5\r\n"));
        }
    }

    #[tokio::test]
    async fn executor_free_owned_connections_support_graceful_watch() {
        for upgrades in [false, true] {
            let (_peer, server) = tokio::io::duplex(64);
            let shutdown = http_transport::adapters::server::graceful::GracefulShutdown::new();
            let task = {
                let builder = http_transport::adapters::server::conn::auto::Builder::new();
                let service = service_fn(|_: Request<Incoming>| async {
                    Ok::<_, Infallible>(Response::new(Empty::<Bytes>::new()))
                });
                if upgrades {
                    tokio::spawn(
                        shutdown.watch(
                            builder
                                .serve_connection_with_upgrades(TokioIo::new(server), service)
                                .into_owned(),
                        ),
                    )
                } else {
                    tokio::spawn(
                        shutdown.watch(
                            builder
                                .serve_connection(TokioIo::new(server), service)
                                .into_owned(),
                        ),
                    )
                }
            };
            assert_eq!(shutdown.count(), 1);
            bounded(shutdown.shutdown()).await;
            let error = bounded(task).await.unwrap().unwrap_err();
            assert_eq!(error.to_string(), "Cancelled");
        }
    }
}
