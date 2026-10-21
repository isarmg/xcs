#[test]
    fn configuration() {
        // One liner.
        auto::Builder::new(TokioExecutor::new())
            .http1()
            .keep_alive(true)
            .http2()
            .keep_alive_interval(None);
        //  .serve_connection(io, service);

        // Using variable.
        let mut builder = auto::Builder::new(TokioExecutor::new());

        builder.http1().keep_alive(true);
        builder.http2().keep_alive_interval(None);
        // builder.serve_connection(io, service);
    }
