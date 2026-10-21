#[cfg(not(miri))]
    #[tokio::test]
    async fn http1_only_fail_if_client_is_http2() {
        let addr = start_server(true, false).await;
        let mut sender = connect_h2(addr).await;

        let _ = sender
            .send_request(Request::new(Empty::<Bytes>::new()))
            .await
            .expect_err("should fail");
    }
