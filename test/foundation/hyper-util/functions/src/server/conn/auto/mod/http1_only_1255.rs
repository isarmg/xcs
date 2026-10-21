#[cfg(not(miri))]
    #[tokio::test]
    async fn http1_only() {
        let addr = start_server(true, false).await;
        let mut sender = connect_h1(addr).await;

        let response = sender
            .send_request(Request::new(Empty::<Bytes>::new()))
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();

        assert_eq!(body, BODY);
    }
