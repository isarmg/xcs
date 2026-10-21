#[cfg(test)]
mod test {
    use super::*;
    use pin_project_lite::pin_project;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pin_project! {
        #[derive(Debug)]
        struct DummyConnection<F> {
            #[pin]
            future: F,
            shutdown_counter: Arc<AtomicUsize>,
        }
    }

    impl<F> private::Sealed for DummyConnection<F> {}

    impl<F: Future> GracefulConnection for DummyConnection<F> {
        type Error = ();

        fn graceful_shutdown(self: Pin<&mut Self>) {
            self.shutdown_counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl<F: Future> Future for DummyConnection<F> {
        type Output = Result<(), ()>;

        fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
            match self.project().future.poll(cx) {
                Poll::Ready(_) => Poll::Ready(Ok(())),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_graceful_shutdown_ok() {
        let graceful = GracefulShutdown::new();
        let shutdown_counter = Arc::new(AtomicUsize::new(0));
        let (dummy_tx, _) = tokio::sync::broadcast::channel(1);

        for i in 1..=3 {
            let mut dummy_rx = dummy_tx.subscribe();
            let shutdown_counter = shutdown_counter.clone();

            let future = async move {
                tokio::time::sleep(std::time::Duration::from_millis(i * 10)).await;
                let _ = dummy_rx.recv().await;
            };
            let dummy_conn = DummyConnection {
                future,
                shutdown_counter,
            };
            let conn = graceful.watch(dummy_conn);
            tokio::spawn(async move {
                conn.await.unwrap();
            });
        }

        assert_eq!(shutdown_counter.load(Ordering::SeqCst), 0);
        let _ = dummy_tx.send(());

        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                panic!("timeout")
            },
            _ = graceful.shutdown() => {
                assert_eq!(shutdown_counter.load(Ordering::SeqCst), 3);
            }
        }
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_graceful_shutdown_delayed_ok() {
        let graceful = GracefulShutdown::new();
        let shutdown_counter = Arc::new(AtomicUsize::new(0));

        for i in 1..=3 {
            let shutdown_counter = shutdown_counter.clone();

            //tokio::time::sleep(std::time::Duration::from_millis(i * 5)).await;
            let future = async move {
                tokio::time::sleep(std::time::Duration::from_millis(i * 50)).await;
            };
            let dummy_conn = DummyConnection {
                future,
                shutdown_counter,
            };
            let conn = graceful.watch(dummy_conn);
            tokio::spawn(async move {
                conn.await.unwrap();
            });
        }

        assert_eq!(shutdown_counter.load(Ordering::SeqCst), 0);

        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                panic!("timeout")
            },
            _ = graceful.shutdown() => {
                assert_eq!(shutdown_counter.load(Ordering::SeqCst), 3);
            }
        }
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_graceful_shutdown_multi_per_watcher_ok() {
        let graceful = GracefulShutdown::new();
        let shutdown_counter = Arc::new(AtomicUsize::new(0));

        for i in 1..=3 {
            let shutdown_counter = shutdown_counter.clone();

            let mut futures = Vec::new();
            for u in 1..=i {
                let future = tokio::time::sleep(std::time::Duration::from_millis(u * 50));
                let dummy_conn = DummyConnection {
                    future,
                    shutdown_counter: shutdown_counter.clone(),
                };
                let conn = graceful.watch(dummy_conn);
                futures.push(conn);
            }
            tokio::spawn(async move {
                futures_util::future::join_all(futures).await;
            });
        }

        assert_eq!(shutdown_counter.load(Ordering::SeqCst), 0);

        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                panic!("timeout")
            },
            _ = graceful.shutdown() => {
                assert_eq!(shutdown_counter.load(Ordering::SeqCst), 6);
            }
        }
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_graceful_shutdown_timeout() {
        let graceful = GracefulShutdown::new();
        let shutdown_counter = Arc::new(AtomicUsize::new(0));

        for i in 1..=3 {
            let shutdown_counter = shutdown_counter.clone();

            let future = async move {
                if i == 1 {
                    std::future::pending::<()>().await
                } else {
                    std::future::ready(()).await
                }
            };
            let dummy_conn = DummyConnection {
                future,
                shutdown_counter,
            };
            let conn = graceful.watch(dummy_conn);
            tokio::spawn(async move {
                conn.await.unwrap();
            });
        }

        assert_eq!(shutdown_counter.load(Ordering::SeqCst), 0);

        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                assert_eq!(shutdown_counter.load(Ordering::SeqCst), 3);
            },
            _ = graceful.shutdown() => {
                panic!("shutdown should not be completed: as not all our conns finish")
            }
        }
    }
}
