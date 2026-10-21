#[cfg(not(miri))]
    #[tokio::test]
    async fn http2() {
        let addr = start_server(false, false).await;
        let mut sender = connect_h2(addr).await;

        let response = sender
            .send_request(Request::new(Empty::<Bytes>::new()))
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();

        assert_eq!(body, BODY);
    }
