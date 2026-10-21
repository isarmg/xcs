#[test]
    #[cfg(feature = "http1")]
    fn preserve_header_case_configuration() {
        // Test preserve_header_case can be set on the main builder
        auto::Builder::new(TokioExecutor::new()).preserve_header_case(true);

        // Can be combined with other configuration
        auto::Builder::new(TokioExecutor::new())
            .preserve_header_case(true)
            .http1_only();
    }
