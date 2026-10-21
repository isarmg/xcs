#[cfg(not(miri))]
    #[tokio::test]
    async fn graceful_shutdown() {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();

        let listener_addr = listener.local_addr().unwrap();

        // Spawn the task in background so that we can connect there
        let listen_task = tokio::spawn(async move { listener.accept().await.unwrap() });
        // Only connect a stream, do not send headers or anything
        let _stream = TcpStream::connect(listener_addr).await.unwrap();

        let (stream, _) = listen_task.await.unwrap();
        let stream = TokioIo::new(stream);
        let builder = auto::Builder::new(TokioExecutor::new());
        let connection = builder.serve_connection(stream, service_fn(hello));

        pin!(connection);

        connection.as_mut().graceful_shutdown();

        let connection_error = tokio::time::timeout(Duration::from_millis(200), connection)
            .await
            .expect("Connection should have finished in a timely manner after graceful shutdown.")
            .expect_err("Connection should have been interrupted.");

        let connection_error = connection_error
            .downcast_ref::<std::io::Error>()
            .expect("The error should have been `std::io::Error`.");
        assert_eq!(connection_error.kind(), std::io::ErrorKind::Interrupted);
    }
