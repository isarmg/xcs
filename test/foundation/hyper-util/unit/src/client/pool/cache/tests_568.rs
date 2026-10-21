#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::task::{self, Poll};

    use futures::future;
    use tower_service::Service;
    use tower_test::assert_request_eq;

    #[tokio::test]
    async fn test_makes_svc_when_empty() {
        let (mock, mut handle) = tower_test::mock::pair();
        let mut cache = super::builder().build(mock);
        handle.allow(1);

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();

        let f = cache.call(1);

        future::join(f, async move {
            assert_request_eq!(handle, 1).send_response("one");
        })
        .await
        .0
        .expect("call");
    }

    #[tokio::test]
    async fn test_reuses_after_idle() {
        let (mock, mut handle) = tower_test::mock::pair();
        let mut cache = super::builder().build(mock);

        // only 1 connection should ever be made
        handle.allow(1);

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let f = cache.call(1);
        let cached = future::join(f, async {
            assert_request_eq!(handle, 1).send_response("one");
        })
        .await
        .0
        .expect("call");
        drop(cached);

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let f = cache.call(1);
        let cached = f.await.expect("call");
        drop(cached);
    }

    // A returned connection is handed to waiters in the order they parked
    // (FIFO), so a waiter cannot be starved by later arrivals.
    #[tokio::test]
    async fn test_waiters_woken_in_fifo_order() {
        use std::task::{Context, Poll, Waker};

        let (mock, mut handle) = tower_test::mock::pair::<u32, &'static str>();
        let mut cache = super::builder().build(mock);
        handle.allow(16);

        // Establish one connection and hold it, so the next checkouts find no
        // idle service and park.
        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let held = future::join(cache.call(0), async {
            assert_request_eq!(handle, 0).send_response("conn");
        })
        .await
        .0
        .expect("call");

        // Park three checkouts in order. Each misses and starts a connect, but
        // the connect is never completed, so each parks on its waiter.
        let mut cx = Context::from_waker(Waker::noop());
        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let mut first = Box::pin(cache.call(1));
        assert!(first.as_mut().poll(&mut cx).is_pending());
        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let mut second = Box::pin(cache.call(2));
        assert!(second.as_mut().poll(&mut cx).is_pending());
        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let mut third = Box::pin(cache.call(3));
        assert!(third.as_mut().poll(&mut cx).is_pending());

        // Returning the connection wakes the oldest waiter first.
        drop(held);
        let first = match first.as_mut().poll(&mut cx) {
            Poll::Ready(r) => r.expect("first"),
            Poll::Pending => panic!("oldest waiter was not woken first"),
        };
        assert!(second.as_mut().poll(&mut cx).is_pending());
        assert!(third.as_mut().poll(&mut cx).is_pending());

        // Returning it again wakes the next-oldest, then the last.
        drop(first);
        let second = match second.as_mut().poll(&mut cx) {
            Poll::Ready(r) => r.expect("second"),
            Poll::Pending => panic!("second waiter was not woken next"),
        };
        assert!(third.as_mut().poll(&mut cx).is_pending());

        drop(second);
        match third.as_mut().poll(&mut cx) {
            Poll::Ready(r) => {
                r.expect("third");
            }
            Poll::Pending => panic!("last waiter was not woken"),
        }
    }

    #[tokio::test]
    async fn dropped_racing_future_cancels_waiter() {
        use std::task::{Context, Poll, Waker};

        let (mock, mut handle) = tower_test::mock::pair::<u32, &'static str>();
        let mut cache = super::builder().build(mock);
        handle.allow(16);

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let held = future::join(cache.call(0), async {
            assert_request_eq!(handle, 0).send_response("conn");
        })
        .await
        .0
        .expect("call");

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let mut dropped = Box::pin(cache.call(1));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(dropped.as_mut().poll(&mut cx).is_pending());
        drop(dropped);

        drop(held);

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let mut reused = Box::pin(cache.call(2));
        match reused.as_mut().poll(&mut cx) {
            Poll::Ready(Ok(cached)) => {
                assert_eq!(*cached.inner(), "conn");
            }
            Poll::Ready(Err(err)) => panic!("unexpected error: {err}"),
            Poll::Pending => panic!("dropped waiter blocked idle reuse"),
        }
    }

    #[tokio::test]
    async fn clone_readiness_reserves_idle_service() {
        let connector = StrictConnector::default();
        let poll_ready_count = connector.poll_ready_count.clone();
        let calls = connector.calls.clone();
        let mut cache = super::builder().build(connector);

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let cached = cache.call(1).await.unwrap();
        assert_eq!(*cached.inner(), 0);
        drop(cached);

        let mut a = cache.clone();
        let mut b = cache.clone();

        std::future::poll_fn(|cx| a.poll_ready(cx)).await.unwrap();
        assert_eq!(poll_ready_count.load(Ordering::SeqCst), 1);
        assert!(!a.is_empty());

        std::future::poll_fn(|cx| b.poll_ready(cx)).await.unwrap();
        assert_eq!(poll_ready_count.load(Ordering::SeqCst), 2);

        let a_cached = a.call(10).await.unwrap();
        assert_eq!(*a_cached.inner(), 0);

        let b_cached = b.call(20).await.unwrap();
        assert_eq!(*b_cached.inner(), 1);

        assert_eq!(*calls.lock().unwrap(), vec![1, 20]);
    }

    #[tokio::test]
    async fn dropped_ready_slot_returns_idle_service() {
        let connector = StrictConnector::default();
        let poll_ready_count = connector.poll_ready_count.clone();
        let mut cache = super::builder().build(connector);

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let cached = cache.call(1).await.unwrap();
        drop(cached);

        let mut clone = cache.clone();
        std::future::poll_fn(|cx| clone.poll_ready(cx))
            .await
            .unwrap();
        drop(clone);

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        assert_eq!(poll_ready_count.load(Ordering::SeqCst), 1);

        let cached = cache.call(2).await.unwrap();
        assert_eq!(*cached.inner(), 0);
    }

    #[tokio::test]
    async fn retain_checks_ready_slot() {
        let connector = StrictConnector::default();
        let poll_ready_count = connector.poll_ready_count.clone();
        let mut cache = super::builder().build(connector);

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let cached = cache.call(1).await.unwrap();
        drop(cached);

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        assert!(!cache.is_empty());

        cache.retain(|svc| *svc != 0);
        assert!(cache.is_empty());

        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        assert_eq!(poll_ready_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn idle_return_wakes_pending_poll_ready() {
        use std::sync::atomic::AtomicBool;
        use std::task::{Context, Waker};

        let connector = PendingConnector::default();
        let allow_ready = connector.allow_ready.clone();
        let mut cache = super::builder().build(connector);

        allow_ready.store(true, Ordering::SeqCst);
        std::future::poll_fn(|cx| cache.poll_ready(cx))
            .await
            .unwrap();
        let held = cache.call(1).await.unwrap();
        assert_eq!(*held.inner(), 0);

        let mut ready = Box::pin(std::future::poll_fn(|cx| cache.poll_ready(cx)));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(ready.as_mut().poll(&mut cx).is_pending());

        drop(held);

        match ready.as_mut().poll(&mut cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(err)) => match err {},
            Poll::Pending => panic!("idle return did not wake pending poll_ready"),
        }
        drop(ready);

        let cached = cache.call(2).await.unwrap();
        assert_eq!(*cached.inner(), 0);

        #[derive(Default)]
        struct PendingConnector {
            allow_ready: Arc<AtomicBool>,
            next: Arc<AtomicUsize>,
            ready: bool,
        }

        impl Service<usize> for PendingConnector {
            type Response = usize;
            type Error = Infallible;
            type Future = std::future::Ready<Result<usize, Infallible>>;

            fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
                if self.allow_ready.swap(false, Ordering::SeqCst) {
                    self.ready = true;
                    Poll::Ready(Ok(()))
                } else {
                    Poll::Pending
                }
            }

            fn call(&mut self, _target: usize) -> Self::Future {
                assert!(self.ready, "connector called without poll_ready");
                self.ready = false;
                let id = self.next.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Ok(id))
            }
        }
    }

    #[derive(Default)]
    struct StrictConnector {
        poll_ready_count: Arc<AtomicUsize>,
        next: Arc<AtomicUsize>,
        calls: Arc<Mutex<Vec<usize>>>,
        ready: bool,
    }

    impl Clone for StrictConnector {
        fn clone(&self) -> Self {
            StrictConnector {
                poll_ready_count: self.poll_ready_count.clone(),
                next: self.next.clone(),
                calls: self.calls.clone(),
                ready: false,
            }
        }
    }

    impl Service<usize> for StrictConnector {
        type Response = usize;
        type Error = Infallible;
        type Future = std::future::Ready<Result<usize, Infallible>>;

        fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.ready = true;
            self.poll_ready_count.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, target: usize) -> Self::Future {
            assert!(self.ready, "connector called without poll_ready");
            self.ready = false;
            self.calls.lock().unwrap().push(target);
            let id = self.next.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok(id))
        }
    }
}
