#[test]
    #[cfg(feature = "http1")]
    fn title_case_headers_configuration() {
        // Test title_case_headers can be set on the main builder
        auto::Builder::new(TokioExecutor::new()).title_case_headers(true);

        // Can be combined with other configuration
        auto::Builder::new(TokioExecutor::new())
            .title_case_headers(true)
            .http1_only();
    }
