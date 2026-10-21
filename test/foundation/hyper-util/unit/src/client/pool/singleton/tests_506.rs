#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::pin::Pin;
    use std::task::Poll;

    use service_stack::Service;

    use super::Singleton;

    #[tokio::test]
    async fn first_call_drives_subsequent_wait() {
        let (mock_svc, mut handle) = tower_test::mock::pair::<(), &'static str>();

        let mut singleton = Singleton::new(mock_svc);

        handle.allow(1);
        std::future::poll_fn(|cx| singleton.poll_ready(cx))
            .await
            .unwrap();
        // First call: should go into Driving
        let fut1 = singleton.call(());
        // Second call: should go into Waiting
        let fut2 = singleton.call(());

        // Expect exactly one request to the inner service
        let ((), send_response) = handle.next_request().await.unwrap();
        send_response.send_response("svc");

        // Both futures should resolve to the same value
        fut1.await.unwrap();
        fut2.await.unwrap();
    }

    #[tokio::test]
    async fn made_state_returns_immediately() {
        let (mock_svc, mut handle) = tower_test::mock::pair::<(), &'static str>();
        let mut singleton = Singleton::new(mock_svc);

        handle.allow(1);
        std::future::poll_fn(|cx| singleton.poll_ready(cx))
            .await
            .unwrap();
        // Drive first call to completion
        let fut1 = singleton.call(());
        let ((), send_response) = handle.next_request().await.unwrap();
        send_response.send_response("svc");
        fut1.await.unwrap();

        // Second call should not hit inner service
        singleton.call(()).await.unwrap();
    }

    #[tokio::test]
    async fn cached_service_poll_ready_error_clears_singleton() {
        // Outer mock returns an inner mock service
        let (outer, mut outer_handle) =
            tower_test::mock::pair::<(), tower_test::mock::Mock<(), &'static str>>();
        let mut singleton = Singleton::new(outer);

        // Allow the singleton to be made
        outer_handle.allow(2);
        std::future::poll_fn(|cx| singleton.poll_ready(cx))
            .await
            .unwrap();

        // First call produces an inner mock service
        let fut1 = singleton.call(());
        let ((), send_inner) = outer_handle.next_request().await.unwrap();
        let (inner, mut inner_handle) = tower_test::mock::pair::<(), &'static str>();
        send_inner.send_response(inner);
        let mut cached = fut1.await.unwrap();

        // Now: allow readiness on the inner mock, then inject error
        inner_handle.allow(1);

        // Inject error so next poll_ready fails
        inner_handle.send_error(std::io::Error::new(
            std::io::ErrorKind::Other,
            "cached poll_ready failed",
        ));

        // Drive poll_ready on cached service
        let err = std::future::poll_fn(|cx| cached.poll_ready(cx))
            .await
            .err()
            .expect("expected poll_ready error");
        assert_eq!(err.to_string(), "cached poll_ready failed");

        // After error, the singleton should be cleared, so a new call drives outer again
        outer_handle.allow(1);
        std::future::poll_fn(|cx| singleton.poll_ready(cx))
            .await
            .unwrap();
        let fut2 = singleton.call(());
        let ((), send_inner2) = outer_handle.next_request().await.unwrap();
        let (inner2, mut inner_handle2) = tower_test::mock::pair::<(), &'static str>();
        send_inner2.send_response(inner2);
        let mut cached2 = fut2.await.unwrap();

        // The new cached service should still work
        inner_handle2.allow(1);
        std::future::poll_fn(|cx| cached2.poll_ready(cx))
            .await
            .expect("expected poll_ready");
        let cfut2 = cached2.call(());
        let ((), send_cached2) = inner_handle2.next_request().await.unwrap();
        send_cached2.send_response("svc2");
        cfut2.await.unwrap();
    }

    #[tokio::test]
    async fn cancel_waiter_does_not_affect_others() {
        let (mock_svc, mut handle) = tower_test::mock::pair::<(), &'static str>();
        let mut singleton = Singleton::new(mock_svc);

        std::future::poll_fn(|cx| singleton.poll_ready(cx))
            .await
            .unwrap();
        let fut1 = singleton.call(());
        let fut2 = singleton.call(());
        drop(fut2); // cancel one waiter

        let ((), send_response) = handle.next_request().await.unwrap();
        send_response.send_response("svc");

        fut1.await.unwrap();
    }

    #[tokio::test]
    async fn maker_error_is_shared_with_waiters() {
        let (mock_svc, mut handle) = tower_test::mock::pair::<(), &'static str>();
        let mut singleton = Singleton::new(mock_svc);

        std::future::poll_fn(|cx| singleton.poll_ready(cx))
            .await
            .unwrap();

        let fut1 = singleton.call(());
        let fut2 = singleton.call(());
        let fut3 = singleton.call(());

        let ((), send_response) = handle.next_request().await.unwrap();
        send_response.send_error(std::io::Error::new(
            std::io::ErrorKind::Other,
            "maker failed",
        ));

        let err1 = fut1.await.unwrap_err();
        let err2 = fut2.await.unwrap_err();
        let err3 = fut3.await.unwrap_err();

        let src1 = err1.source().expect("driver source");
        let src2 = err2.source().expect("waiter source");
        let src3 = err3.source().expect("waiter source");

        assert_eq!(src1.to_string(), "maker failed");
        assert!(std::ptr::addr_eq(src1, src2));
        assert!(std::ptr::addr_eq(src1, src3));
    }

    #[tokio::test]
    async fn cancel_driver_hands_off_to_waiter() {
        let (mock_svc, mut handle) = tower_test::mock::pair::<(), &'static str>();
        let mut singleton = Singleton::new(mock_svc);

        std::future::poll_fn(|cx| singleton.poll_ready(cx))
            .await
            .unwrap();
        let mut fut1 = singleton.call(());
        let fut2 = singleton.call(());

        // poll driver just once, and then drop
        std::future::poll_fn(move |cx| {
            let _ = Pin::new(&mut fut1).poll(cx);
            Poll::Ready(())
        })
        .await;

        let ((), send_response) = handle.next_request().await.unwrap();
        send_response.send_response("svc");

        fut2.await.unwrap();
    }

    #[tokio::test]
    async fn cancel_driver_promotes_parked_waiter() {
        let (mock_svc, mut handle) = tower_test::mock::pair::<(), &'static str>();
        let mut singleton = Singleton::new(mock_svc);

        std::future::poll_fn(|cx| singleton.poll_ready(cx))
            .await
            .unwrap();
        let mut fut1 = singleton.call(());
        let fut2 = singleton.call(());

        // Start the make future so dropping fut1 below exercises driver
        // handoff during an in-flight connection attempt.
        std::future::poll_fn(|cx| {
            assert!(Pin::new(&mut fut1).poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;

        // Poll the waiter once so it parks and stores a waker in the batch.
        // This covers promotion of an already-parked waiter, not just a
        // registered-but-never-polled waiter.
        let mut waiter = tokio_test::task::spawn(fut2);
        assert!(waiter.poll().is_pending());
        assert!(!waiter.is_woken());

        // When the driver is dropped, the promoted parked waiter must be
        // woken so it can take over driving the shared make future.
        drop(fut1);
        assert!(waiter.is_woken());

        // Poll after promotion, before the maker responds, so the waiter
        // actually takes over as driver and stores its own waker.
        assert!(waiter.poll().is_pending());

        let ((), send_response) = handle.next_request().await.unwrap();
        send_response.send_response("svc");

        assert!(waiter.is_woken());
        match waiter.poll() {
            Poll::Ready(Ok(_)) => {}
            other => panic!("expected promoted waiter to complete, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn cancel_all_waiters_clears_singleton() {
        let (mock_svc, _handle) = tower_test::mock::pair::<(), &'static str>();
        let mut singleton = Singleton::new(mock_svc);

        std::future::poll_fn(|cx| singleton.poll_ready(cx))
            .await
            .unwrap();
        let fut1 = singleton.call(());
        let fut2 = singleton.call(());

        drop(fut1);
        drop(fut2);

        assert!(singleton.is_empty());
    }

    #[tokio::test]
    async fn cancel_non_driver_waiter_does_not_block_others() {
        let (mock_svc, mut handle) = tower_test::mock::pair::<(), &'static str>();
        let mut singleton = Singleton::new(mock_svc);

        std::future::poll_fn(|cx| singleton.poll_ready(cx))
            .await
            .unwrap();
        let fut1 = singleton.call(());
        let fut2 = singleton.call(());
        let fut3 = singleton.call(());
        drop(fut2);

        let ((), send_response) = handle.next_request().await.unwrap();
        send_response.send_response("svc");

        fut1.await.unwrap();
        fut3.await.unwrap();
    }
}
