#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sync_capture_connection() {
        let (tx, rx) = CaptureConnection::new();
        assert!(
            rx.connection_metadata().is_none(),
            "connection has not been set"
        );
        tx.set(&Connected::new().proxy(true));
        assert!(
            rx.connection_metadata()
                .as_ref()
                .expect("connected should be set")
                .is_proxied()
        );

        // ensure it can be called multiple times
        assert!(
            rx.connection_metadata()
                .as_ref()
                .expect("connected should be set")
                .is_proxied()
        );
    }

    #[tokio::test]
    async fn async_capture_connection() {
        let (tx, mut rx) = CaptureConnection::new();
        assert!(
            rx.connection_metadata().is_none(),
            "connection has not been set"
        );
        let test_task = tokio::spawn(async move {
            assert!(
                rx.wait_for_connection_metadata()
                    .await
                    .as_ref()
                    .expect("connection should be set")
                    .is_proxied()
            );
            // can be awaited multiple times
            assert!(
                rx.wait_for_connection_metadata().await.is_some(),
                "should be awaitable multiple times"
            );

            assert!(rx.connection_metadata().is_some());
        });
        // can't be finished, we haven't set the connection yet
        assert!(!test_task.is_finished());
        tx.set(&Connected::new().proxy(true));

        assert!(test_task.await.is_ok());
    }

    #[tokio::test]
    async fn capture_connection_sender_side_dropped() {
        let (tx, mut rx) = CaptureConnection::new();
        assert!(
            rx.connection_metadata().is_none(),
            "connection has not been set"
        );
        drop(tx);
        assert!(rx.wait_for_connection_metadata().await.is_none());
    }
}
