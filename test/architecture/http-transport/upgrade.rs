// HTTP/1 upgrade transport contract, not a WebSocket implementation.
mod upgrade_transport {
    use http_body::Empty;
    use http_transport::{
        adapters::rt::TokioIo,
        body::{Bytes, Incoming},
        service::service_fn,
        Request, Response, StatusCode,
    };
    use std::{convert::Infallible, time::Duration};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn http1_upgrade_preserves_buffered_bytes_and_bidirectional_io() {
        tokio::time::timeout(Duration::from_secs(5), async {
            let (mut client, server) = tokio::io::duplex(4096);
            let (completed, completion) = tokio::sync::oneshot::channel();
            let completed = std::sync::Arc::new(std::sync::Mutex::new(Some(completed)));
            let task = tokio::spawn(async move {
                http_transport::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(server), service_fn(move |mut request: Request<Incoming>| {
                        let completed = completed.lock().unwrap().take().unwrap();
                        async move {
                            assert_eq!(request.version(), http_transport::Version::HTTP_11);
                            assert_eq!(request.method(), "GET");
                            assert_eq!(request.headers()["upgrade"], "test-stream");
                            let upgrade = http_transport::upgrade::on(&mut request);
                            tokio::spawn(async move {
                                let mut stream = TokioIo::new(upgrade.await.unwrap());
                                let mut payload = [0; 5];
                                stream.read_exact(&mut payload).await.unwrap();
                                assert_eq!(&payload, b"early");
                                stream.write_all(b"ready").await.unwrap();
                                stream.read_exact(&mut payload).await.unwrap();
                                assert_eq!(&payload, b"later");
                                stream.write_all(b"done!").await.unwrap();
                                stream.shutdown().await.unwrap();
                                completed.send(()).unwrap();
                            });
                            Ok::<_, Infallible>(Response::builder()
                                .status(StatusCode::SWITCHING_PROTOCOLS)
                                .header("connection", "upgrade")
                                .header("upgrade", "test-stream")
                                .body(Empty::<Bytes>::new()).unwrap())
                        }
                    })).with_upgrades().await.unwrap();
            });
            // Send bytes beyond the HTTP head in the same write. The upgrade
            // must retain bytes already read by the HTTP parser.
            client.write_all(b"GET / HTTP/1.1\r\nHost: local.test\r\nConnection: upgrade\r\nUpgrade: test-stream\r\n\r\nearly").await.unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                assert!(head.len() < 4096, "response head must terminate");
                head.push(client.read_u8().await.unwrap());
            }
            let head = String::from_utf8(head).unwrap().to_ascii_lowercase();
            assert!(head.starts_with("http/1.1 101 switching protocols\r\n"));
            assert!(head.contains("\r\nconnection: upgrade\r\n"));
            assert!(head.contains("\r\nupgrade: test-stream\r\n"));
            let mut payload = [0; 5];
            client.read_exact(&mut payload).await.unwrap();
            assert_eq!(&payload, b"ready");
            client.write_all(b"later").await.unwrap();
            client.read_exact(&mut payload).await.unwrap();
            assert_eq!(&payload, b"done!");
            assert_eq!(client.read(&mut payload).await.unwrap(), 0);
            completion.await.unwrap();
            task.await.unwrap();
        }).await.expect("HTTP/1 upgrade must complete without a socket or hangs");
    }
    #[tokio::test]
    async fn auto_upgrade_preserves_buffered_bytes_and_bidirectional_io() {
        tokio::time::timeout(Duration::from_secs(5), async {
            let (mut client, server) = tokio::io::duplex(4096);
            let (completed, completion) = tokio::sync::oneshot::channel();
            let completed = std::sync::Arc::new(std::sync::Mutex::new(Some(completed)));
            let task = tokio::spawn(async move {
                http_transport::adapters::server::conn::auto::Builder::new()
                    .serve_connection_with_upgrades(TokioIo::new(server), service_fn(move |mut request: Request<Incoming>| {
                        let completed = completed.lock().unwrap().take().unwrap();
                        async move {
                            assert_eq!(request.version(), http_transport::Version::HTTP_11);
                            assert_eq!(request.method(), "GET");
                            assert_eq!(request.headers()["upgrade"], "test-stream");
                            let upgrade = http_transport::upgrade::on(&mut request);
                            tokio::spawn(async move {
                                let mut stream = TokioIo::new(upgrade.await.unwrap());
                                let mut payload = [0; 5];
                                stream.read_exact(&mut payload).await.unwrap();
                                assert_eq!(&payload, b"early");
                                stream.write_all(b"ready").await.unwrap();
                                stream.read_exact(&mut payload).await.unwrap();
                                assert_eq!(&payload, b"later");
                                stream.write_all(b"done!").await.unwrap();
                                stream.shutdown().await.unwrap();
                                completed.send(()).unwrap();
                            });
                            Ok::<_, Infallible>(Response::builder()
                                .status(StatusCode::SWITCHING_PROTOCOLS)
                                .header("connection", "upgrade")
                                .header("upgrade", "test-stream")
                                .body(Empty::<Bytes>::new()).unwrap())
                        }
                    })).await.unwrap();
            });
            // Send bytes beyond the HTTP head in the same write. The upgrade
            // must retain bytes already read by the HTTP parser.
            client.write_all(b"GET / HTTP/1.1\r\nHost: local.test\r\nConnection: upgrade\r\nUpgrade: test-stream\r\n\r\nearly").await.unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                assert!(head.len() < 4096, "response head must terminate");
                head.push(client.read_u8().await.unwrap());
            }
            let head = String::from_utf8(head).unwrap().to_ascii_lowercase();
            assert!(head.starts_with("http/1.1 101 switching protocols\r\n"));
            assert!(head.contains("\r\nconnection: upgrade\r\n"));
            assert!(head.contains("\r\nupgrade: test-stream\r\n"));
            let mut payload = [0; 5];
            client.read_exact(&mut payload).await.unwrap();
            assert_eq!(&payload, b"ready");
            client.write_all(b"later").await.unwrap();
            client.read_exact(&mut payload).await.unwrap();
            assert_eq!(&payload, b"done!");
            assert_eq!(client.read(&mut payload).await.unwrap(), 0);
            completion.await.unwrap();
            task.await.unwrap();
        }).await.expect("HTTP/1 upgrade must complete without a socket or hangs");
    }
    #[tokio::test]
    async fn http1_transport_rejects_http2_wire_version_before_service() {
        tokio::time::timeout(Duration::from_secs(5), async {
            let (mut client, server) = tokio::io::duplex(4096);
            let task = tokio::spawn(async move {
                http_transport::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(server), service_fn(|_: Request<Incoming>| async {
                        panic!("unsupported wire version must not reach application service");
                        #[allow(unreachable_code)]
                        Ok::<_, Infallible>(Response::new(Empty::<Bytes>::new()))
                    })).with_upgrades().await
            });
            client.write_all(b"GET / HTTP/2.0\r\nHost: local.test\r\nConnection: upgrade\r\nUpgrade: test-stream\r\n\r\n").await.unwrap();
            client.shutdown().await.unwrap();
            let error = task.await.unwrap().unwrap_err();
            assert!(error.is_parse());
            let mut response = Vec::new();
            client.read_to_end(&mut response).await.unwrap();
            assert!(response.starts_with(b"HTTP/1.1 400"), "{response:?}");
        }).await.expect("unsupported wire version must fail promptly");
    }
}
