// Protocol detection shares the first HTTP/1 header deadline; no sockets or wall-clock waits.
mod probe_timeout {
    use http_body::Empty;
    use http_transport::{
        adapters::{
            rt::{TokioIo, TokioTimer},
            server::conn::auto::Builder,
        },
        body::{Bytes, Incoming},
        service::service_fn,
        Request, Response,
    };
    use std::{
        convert::Infallible,
        future::{poll_fn, Future},
        pin::Pin,
        task::Poll,
        time::Duration,
    };
    use tokio::io::AsyncWriteExt;
    type Connection =
        Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>;
    fn connection(builder: &Builder, io: tokio::io::DuplexStream, upgrades: bool) -> Connection {
        let service = service_fn(|_: Request<Incoming>| async {
            Ok::<_, Infallible>(Response::new(Empty::<Bytes>::new()))
        });
        if upgrades {
            Box::pin(
                builder
                    .serve_connection_with_upgrades(TokioIo::new(io), service)
                    .into_owned(),
            )
        } else {
            Box::pin(
                builder
                    .serve_connection(TokioIo::new(io), service)
                    .into_owned(),
            )
        }
    }
    async fn pending(conn: &mut Connection) {
        poll_fn(|cx| {
            assert!(conn.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
    }
    async fn timed_out(conn: &mut Connection) {
        poll_fn(|cx| {
            let Poll::Ready(Err(error)) = conn.as_mut().poll(cx) else {
                panic!("header deadline must already have expired")
            };
            assert!(error
                .downcast_ref::<http_transport::Error>()
                .unwrap()
                .is_timeout());
            Poll::Ready(())
        })
        .await;
    }
    fn builder() -> Builder {
        let mut b = Builder::new();
        b.http1()
            .timer(TokioTimer::new())
            .header_read_timeout(Duration::from_secs(5));
        b
    }
    #[tokio::test(start_paused = true)]
    async fn stalled_protocol_prefix_obeys_header_timeout() {
        for upgrades in [false, true] {
            for prefix in [b"".as_slice(), b"P", b"PRI * HTTP/2.0\r\n"] {
                let (mut peer, io) = tokio::io::duplex(1024);
                peer.write_all(prefix).await.unwrap();
                let mut conn = connection(&builder(), io, upgrades);
                // Constructing an owned connection does not start its deadline.
                tokio::time::advance(Duration::from_secs(20)).await;
                pending(&mut conn).await;
                tokio::time::advance(Duration::from_secs(6)).await;
                timed_out(&mut conn).await;
            }
        }
    }
    #[tokio::test(start_paused = true)]
    async fn probe_to_parser_keeps_original_deadline() {
        for upgrades in [false, true] {
            let (mut peer, io) = tokio::io::duplex(1024);
            peer.write_all(b"P").await.unwrap();
            let mut conn = connection(&builder(), io, upgrades);
            pending(&mut conn).await;
            tokio::time::advance(Duration::from_secs(3)).await;
            peer.write_all(b"OST / HTTP/1.1\r\nHost:").await.unwrap();
            pending(&mut conn).await;
            tokio::time::advance(Duration::from_secs(3)).await;
            timed_out(&mut conn).await;
        }
    }
    #[tokio::test(start_paused = true)]
    async fn later_keepalive_header_receives_fresh_deadline() {
        for upgrades in [false, true] {
            let (mut peer, io) = tokio::io::duplex(1024);
            peer.write_all(b"P").await.unwrap();
            let mut conn = connection(&builder(), io, upgrades);
            pending(&mut conn).await;
            tokio::time::advance(Duration::from_secs(3)).await;
            peer.write_all(b"OST / HTTP/1.1\r\nHost: local.test\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
            pending(&mut conn).await;
            // Begin another header; its deadline must not inherit the first one.
            peer.write_all(b"G").await.unwrap();
            pending(&mut conn).await;
            tokio::time::advance(Duration::from_secs(3)).await;
            pending(&mut conn).await;
            tokio::time::advance(Duration::from_secs(3)).await;
            timed_out(&mut conn).await;
        }
    }
    #[tokio::test(start_paused = true)]
    async fn absent_timer_and_disabled_timeout_remain_unbounded() {
        for upgrades in [false, true] {
            for explicit_none in [false, true] {
                let (mut peer, io) = tokio::io::duplex(1024);
                peer.write_all(b"P").await.unwrap();
                let mut b = Builder::new();
                if explicit_none {
                    b.http1().timer(TokioTimer::new()).header_read_timeout(None);
                }
                let mut conn = connection(&b, io, upgrades);
                pending(&mut conn).await;
                tokio::time::advance(Duration::from_secs(60)).await;
                pending(&mut conn).await;
            }
        }
    }
    #[tokio::test(start_paused = true)]
    async fn installed_timer_uses_existing_default_header_timeout() {
        for upgrades in [false, true] {
            let (mut peer, io) = tokio::io::duplex(1024);
            peer.write_all(b"P").await.unwrap();
            let mut b = Builder::new();
            b.http1().timer(TokioTimer::new());
            let mut conn = connection(&b, io, upgrades);
            pending(&mut conn).await;
            tokio::time::advance(Duration::from_secs(29)).await;
            pending(&mut conn).await;
            tokio::time::advance(Duration::from_secs(2)).await;
            timed_out(&mut conn).await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn timed_probe_rejects_every_fragmented_h2_preface_without_response() {
        use tokio::io::AsyncReadExt;
        let preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        for upgrades in [false, true] {
            for split in 1..preface.len() {
                let (mut peer, io) = tokio::io::duplex(1024);
                peer.write_all(&preface[..split]).await.unwrap();
                let mut conn = connection(&builder(), io, upgrades);
                pending(&mut conn).await;
                tokio::time::advance(Duration::from_secs(1)).await;
                peer.write_all(&preface[split..]).await.unwrap();
                let error = conn.await.unwrap_err();
                assert_eq!(error.to_string(), "HTTP/2 is not supported");
                let mut wire = Vec::new();
                peer.read_to_end(&mut wire).await.unwrap();
                assert!(wire.is_empty());
            }
        }
    }
}

mod routing_header_timeout {
    use std::{
        io,
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    };
    use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, DuplexStream, ReadBuf};
    struct ObservedIo {
        io: DuplexStream,
        first_read: Option<tokio::sync::oneshot::Sender<()>>,
    }
    impl AsyncRead for ObservedIo {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            if let Some(tx) = self.first_read.take() {
                let _ = tx.send(());
            }
            Pin::new(&mut self.io).poll_read(cx, buf)
        }
    }
    impl AsyncWrite for ObservedIo {
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
    struct OnceListener(Option<ObservedIo>);
    impl http_routing::serve::Listener for OnceListener {
        type Io = ObservedIo;
        type Addr = ();
        async fn accept(&mut self) -> (Self::Io, Self::Addr) {
            match self.0.take() {
                Some(io) => (io, ()),
                None => std::future::pending().await,
            }
        }
        fn local_addr(&self) -> io::Result<()> {
            Ok(())
        }
    }
    #[tokio::test(start_paused = true)]
    async fn maintained_routing_serve_times_out_stalled_prefix() {
        let (mut peer, io) = tokio::io::duplex(1024);
        peer.write_all(b"P").await.unwrap();
        let (read_tx, read_rx) = tokio::sync::oneshot::channel();
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        let listener = OnceListener(Some(ObservedIo {
            io,
            first_read: Some(read_tx),
        }));
        let app = http_routing::Router::new().route(
            "/",
            http_routing::routing::get(|| async {
                panic!("stalled header must not reach routing");
                #[allow(unreachable_code)]
                "unexpected"
            }),
        );
        let task = tokio::spawn(async move {
            http_routing::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stop_rx.await;
                })
                .await
                .unwrap();
        });
        read_rx.await.unwrap();
        tokio::time::advance(Duration::from_secs(29)).await;
        std::future::poll_fn(|cx| {
            let mut bytes = [0; 1];
            let mut buf = ReadBuf::new(&mut bytes);
            assert!(Pin::new(&mut peer).poll_read(cx, &mut buf).is_pending());
            Poll::Ready(())
        })
        .await;
        tokio::time::advance(Duration::from_secs(2)).await;
        tokio::task::yield_now().await;
        std::future::poll_fn(|cx| {
            let mut bytes = [0; 1];
            let mut buf = ReadBuf::new(&mut bytes);
            assert!(matches!(
                Pin::new(&mut peer).poll_read(cx, &mut buf),
                Poll::Ready(Ok(()))
            ));
            assert!(buf.filled().is_empty());
            Poll::Ready(())
        })
        .await;
        stop_tx.send(()).unwrap();
        task.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn maintained_routing_serve_allows_upload_body_beyond_header_deadline() {
        use http_body::BodyExt;
        use tokio::io::AsyncReadExt;

        let (mut peer, io) = tokio::io::duplex(1024);
        let payload: Vec<u8> = (0..65536).map(|index| (index % 251) as u8).collect();
        let first_chunk = 32;
        peer.write_all(format!(
            "POST /upload HTTP/1.1\r\nHost: local.test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            payload.len()
        ).as_bytes()).await.unwrap();
        peer.write_all(&payload[..first_chunk]).await.unwrap();
        let (body_started_tx, body_started_rx) = tokio::sync::oneshot::channel();
        let body_started = std::sync::Arc::new(std::sync::Mutex::new(Some(body_started_tx)));
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        let listener = OnceListener(Some(ObservedIo {
            io,
            first_read: None,
        }));
        let app = http_routing::Router::new().route(
            "/upload",
            http_routing::routing::post(move |request: http_routing::extract::Request| {
                let started = body_started.lock().unwrap().take().unwrap();
                async move {
                    let mut body = request.into_body();
                    let first = body.frame().await.unwrap().unwrap().into_data().unwrap();
                    assert_eq!(first.len(), first_chunk);
                    // Synchronize after headers are parsed AND partial body data is consumed.
                    started.send(()).unwrap();
                    let mut received = first.to_vec();
                    received.extend_from_slice(&body.collect().await.unwrap().to_bytes());
                    received
                }
            }),
        );
        let task = tokio::spawn(async move {
            http_routing::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stop_rx.await;
                })
                .await
                .unwrap();
        });
        body_started_rx.await.unwrap();
        tokio::time::advance(Duration::from_secs(61)).await;
        tokio::task::yield_now().await;
        // No early response or EOF: the header deadline must not become a body timeout.
        std::future::poll_fn(|cx| {
            let mut bytes = [0; 1];
            let mut buf = ReadBuf::new(&mut bytes);
            assert!(Pin::new(&mut peer).poll_read(cx, &mut buf).is_pending());
            Poll::Ready(())
        })
        .await;
        peer.write_all(&payload[first_chunk..]).await.unwrap();
        let mut response = Vec::new();
        peer.read_to_end(&mut response).await.unwrap();
        let header_end = response
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap()
            + 4;
        assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
        let headers = std::str::from_utf8(&response[..header_end])
            .unwrap()
            .to_ascii_lowercase();
        assert!(headers.contains("\r\ncontent-length: 65536\r\n"));
        assert_eq!(&response[header_end..], payload.as_slice());
        stop_tx.send(()).unwrap();
        task.await.unwrap();
    }

    struct SlowDownload {
        first: Option<http_transport::body::Bytes>,
        rest: Option<http_transport::body::Bytes>,
        release: tokio::sync::oneshot::Receiver<()>,
    }
    impl http_body::Body for SlowDownload {
        type Data = http_transport::body::Bytes;
        type Error = std::convert::Infallible;
        fn poll_frame(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
            use std::future::Future;
            if let Some(first) = self.first.take() {
                return Poll::Ready(Some(Ok(http_body::Frame::data(first))));
            }
            if self.rest.is_none() {
                return Poll::Ready(None);
            }
            if Pin::new(&mut self.release).poll(cx).is_pending() {
                return Poll::Pending;
            }
            Poll::Ready(Some(Ok(http_body::Frame::data(self.rest.take().unwrap()))))
        }
        fn size_hint(&self) -> http_body::SizeHint {
            http_body::SizeHint::with_exact(
                (self.first.as_ref().map_or(0, |bytes| bytes.len())
                    + self.rest.as_ref().map_or(0, |bytes| bytes.len())) as u64,
            )
        }
    }

    #[tokio::test(start_paused = true)]
    async fn maintained_routing_serve_allows_streamed_download_beyond_header_deadline() {
        use tokio::io::AsyncReadExt;
        let (mut peer, io) = tokio::io::duplex(1024);
        let payload: Vec<u8> = (0..65536).map(|index| (index % 251) as u8).collect();
        let first_chunk = 32;
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let body = std::sync::Arc::new(std::sync::Mutex::new(Some(SlowDownload {
            first: Some(http_transport::body::Bytes::copy_from_slice(
                &payload[..first_chunk],
            )),
            rest: Some(http_transport::body::Bytes::copy_from_slice(
                &payload[first_chunk..],
            )),
            release: release_rx,
        })));
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        let listener = OnceListener(Some(ObservedIo {
            io,
            first_read: None,
        }));
        let app = http_routing::Router::new().route(
            "/download",
            http_routing::routing::get(move || {
                let body = body.lock().unwrap().take().unwrap();
                async move { http_routing::body::Body::new(body) }
            }),
        );
        let task = tokio::spawn(async move {
            http_routing::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stop_rx.await;
                })
                .await
                .unwrap();
        });
        peer.write_all(b"GET /download HTTP/1.1\r\nHost: local.test\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            assert!(head.len() < 4096, "response headers must terminate");
            head.push(peer.read_u8().await.unwrap());
        }
        assert!(head.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(std::str::from_utf8(&head)
            .unwrap()
            .to_ascii_lowercase()
            .contains("\r\ncontent-length: 65536\r\n"));
        let mut received = vec![0; first_chunk];
        peer.read_exact(&mut received).await.unwrap();
        assert_eq!(received, payload[..first_chunk]);
        // Synchronize on actual response headers and first body bytes received by the peer.
        tokio::time::advance(Duration::from_secs(61)).await;
        tokio::task::yield_now().await;
        std::future::poll_fn(|cx| {
            let mut bytes = [0; 1];
            let mut buf = ReadBuf::new(&mut bytes);
            assert!(Pin::new(&mut peer).poll_read(cx, &mut buf).is_pending());
            Poll::Ready(())
        })
        .await;
        release_tx.send(()).unwrap();
        peer.read_to_end(&mut received).await.unwrap();
        assert_eq!(received, payload);
        stop_tx.send(()).unwrap();
        task.await.unwrap();
    }
}
