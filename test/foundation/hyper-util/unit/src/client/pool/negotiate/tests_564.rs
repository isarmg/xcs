#[cfg(test)]
mod tests {
    use futures::future;
    use tower_service::Service;
    use tower_test::assert_request_eq;

    #[tokio::test]
    async fn not_negotiated_falls_back_to_left() {
        let (mock_svc, mut handle) = tower_test::mock::pair::<(), &'static str>();

        let mut negotiate = super::builder()
            .connect(mock_svc)
            .inspect(|_: &&str| false)
            .fallback(layer_fn(|s| s))
            .upgrade(layer_fn(|s| s))
            .build();

        std::future::poll_fn(|cx| negotiate.poll_ready(cx))
            .await
            .unwrap();

        let fut = negotiate.call(());
        let nsvc = future::join(fut, async move {
            assert_request_eq!(handle, ()).send_response("one");
        })
        .await
        .0
        .expect("call");
        assert!(nsvc.is_fallback());
    }

    #[tokio::test]
    async fn negotiated_uses_right() {
        let (mock_svc, mut handle) = tower_test::mock::pair::<(), &'static str>();

        let mut negotiate = super::builder()
            .connect(mock_svc)
            .inspect(|_: &&str| true)
            .fallback(layer_fn(|s| s))
            .upgrade(layer_fn(|s| s))
            .build();

        std::future::poll_fn(|cx| negotiate.poll_ready(cx))
            .await
            .unwrap();

        let fut = negotiate.call(());
        let nsvc = future::join(fut, async move {
            assert_request_eq!(handle, ()).send_response("one");
        })
        .await
        .0
        .expect("call");

        assert!(nsvc.is_upgraded());
    }

    fn layer_fn<F>(f: F) -> LayerFn<F> {
        LayerFn(f)
    }

    #[derive(Clone)]
    struct LayerFn<F>(F);

    impl<F, S, Out> tower_layer::Layer<S> for LayerFn<F>
    where
        F: Fn(S) -> Out,
    {
        type Service = Out;
        fn layer(&self, inner: S) -> Self::Service {
            (self.0)(inner)
        }
    }
}
