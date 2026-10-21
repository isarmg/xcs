#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::hash::Hash;
    use std::pin::Pin;
    use std::task::{self, Poll};
    use std::time::Duration;

    use super::{Connecting, Key, Pool, Poolable, Reservation, WeakOpt};
    use crate::rt::{TokioExecutor, TokioTimer};

    use crate::common::timer;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct KeyImpl(http::uri::Scheme, http::uri::Authority);

    type KeyTuple = (http::uri::Scheme, http::uri::Authority);

    /// Test unique reservations.
    #[derive(Debug, PartialEq, Eq)]
    struct Uniq<T>(T);

    impl<T: Send + 'static + Unpin> Poolable for Uniq<T> {
        fn is_open(&self) -> bool {
            true
        }

        fn reserve(self) -> Reservation<Self> {
            Reservation::Unique(self)
        }

        fn can_share(&self) -> bool {
            false
        }
    }

    fn c<T: Poolable, K: Key>(key: K) -> Connecting<T, K> {
        Connecting {
            key,
            pool: WeakOpt::none(),
        }
    }

    fn host_key(s: &str) -> KeyImpl {
        KeyImpl(http::uri::Scheme::HTTP, s.parse().expect("host key"))
    }

    fn pool_no_timer<T, K: Key>() -> Pool<T, K> {
        pool_max_idle_no_timer(usize::MAX)
    }

    fn pool_max_idle_no_timer<T, K: Key>(max_idle: usize) -> Pool<T, K> {
        let pool = Pool::new(
            super::Config {
                idle_timeout: Some(Duration::from_millis(100)),
                max_idle_per_host: max_idle,
            },
            TokioExecutor::new(),
            Option::<timer::Timer>::None,
        );
        pool.no_timer();
        pool
    }

    #[tokio::test]
    async fn test_pool_checkout_smoke() {
        let pool = pool_no_timer();
        let key = host_key("foo");
        let pooled = pool.pooled(c(key.clone()), Uniq(41));

        drop(pooled);

        match pool.checkout(key).await {
            Ok(pooled) => assert_eq!(*pooled, Uniq(41)),
            Err(_) => panic!("not ready"),
        };
    }

    /// Helper to check if the future is ready after polling once.
    struct PollOnce<'a, F>(&'a mut F);

    impl<F, T, U> Future for PollOnce<'_, F>
    where
        F: Future<Output = Result<T, U>> + Unpin,
    {
        type Output = Option<()>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
            match Pin::new(&mut self.0).poll(cx) {
                Poll::Ready(Ok(_)) => Poll::Ready(Some(())),
                Poll::Ready(Err(_)) => Poll::Ready(Some(())),
                Poll::Pending => Poll::Ready(None),
            }
        }
    }

    #[tokio::test]
    async fn test_pool_checkout_returns_none_if_expired() {
        let pool = pool_no_timer();
        let key = host_key("foo");
        let pooled = pool.pooled(c(key.clone()), Uniq(41));

        drop(pooled);
        tokio::time::sleep(pool.locked().timeout.unwrap()).await;
        let mut checkout = pool.checkout(key);
        let poll_once = PollOnce(&mut checkout);
        let is_not_ready = poll_once.await.is_none();
        assert!(is_not_ready);
    }

    #[tokio::test]
    async fn test_pool_checkout_removes_expired() {
        let pool = pool_no_timer();
        let key = host_key("foo");

        pool.pooled(c(key.clone()), Uniq(41));
        pool.pooled(c(key.clone()), Uniq(5));
        pool.pooled(c(key.clone()), Uniq(99));

        assert_eq!(
            pool.locked().idle.get(&key).map(|entries| entries.len()),
            Some(3)
        );
        tokio::time::sleep(pool.locked().timeout.unwrap()).await;

        let mut checkout = pool.checkout(key.clone());
        let poll_once = PollOnce(&mut checkout);
        // checkout.await should clean out the expired
        poll_once.await;
        assert!(!pool.locked().idle.contains_key(&key));
    }

    #[test]
    fn test_pool_max_idle_per_host() {
        let pool = pool_max_idle_no_timer(2);
        let key = host_key("foo");

        pool.pooled(c(key.clone()), Uniq(41));
        pool.pooled(c(key.clone()), Uniq(5));
        pool.pooled(c(key.clone()), Uniq(99));

        // pooled and dropped 3, max_idle should only allow 2
        assert_eq!(
            pool.locked().idle.get(&key).map(|entries| entries.len()),
            Some(2)
        );
    }

    #[tokio::test]
    async fn test_pool_timer_removes_expired_realtime() {
        test_pool_timer_removes_expired_inner().await
    }

    #[tokio::test(start_paused = true)]
    async fn test_pool_timer_removes_expired_faketime() {
        test_pool_timer_removes_expired_inner().await
    }

    async fn test_pool_timer_removes_expired_inner() {
        let pool = Pool::new(
            super::Config {
                idle_timeout: Some(Duration::from_millis(10)),
                max_idle_per_host: usize::MAX,
            },
            TokioExecutor::new(),
            Some(TokioTimer::new()),
        );

        let key = host_key("foo");

        pool.pooled(c(key.clone()), Uniq(41));
        pool.pooled(c(key.clone()), Uniq(5));
        pool.pooled(c(key.clone()), Uniq(99));

        assert_eq!(
            pool.locked().idle.get(&key).map(|entries| entries.len()),
            Some(3)
        );
        assert!(pool.locked().idle_interval_ref.is_some());

        // Let the timer tick passed the expiration...
        tokio::time::sleep(Duration::from_millis(30)).await;

        // But minimum interval is higher, so nothing should have been reaped
        assert_eq!(
            pool.locked().idle.get(&key).map(|entries| entries.len()),
            Some(3)
        );
        assert!(pool.locked().idle_interval_ref.is_some());

        // Now wait passed the minimum interval more
        tokio::time::sleep(Duration::from_millis(70)).await;
        // Yield in case other task hasn't been able to run :shrug:
        tokio::task::yield_now().await;

        assert!(!pool.locked().idle.contains_key(&key));
        assert!(pool.locked().idle_interval_ref.is_none());

        // Insert new key and check timer is recreated
        pool.pooled(c(key.clone()), Uniq(7));
        assert!(pool.locked().idle_interval_ref.is_some());
    }

    #[tokio::test]
    async fn test_pool_checkout_task_unparked() {
        use futures::FutureExt;
        use futures::future::join;

        let pool = pool_no_timer();
        let key = host_key("foo");
        let pooled = pool.pooled(c(key.clone()), Uniq(41));

        let checkout = join(pool.checkout(key), async {
            // the checkout future will park first,
            // and then this lazy future will be polled, which will insert
            // the pooled back into the pool
            //
            // this test makes sure that doing so will unpark the checkout
            drop(pooled);
        })
        .map(|(entry, _)| entry);

        assert_eq!(*checkout.await.unwrap(), Uniq(41));
    }

    #[tokio::test]
    async fn test_pool_checkout_drop_cleans_up_waiters() {
        let pool = pool_no_timer::<Uniq<i32>, KeyImpl>();
        let key = host_key("foo");

        let mut checkout1 = pool.checkout(key.clone());
        let mut checkout2 = pool.checkout(key.clone());

        let poll_once1 = PollOnce(&mut checkout1);
        let poll_once2 = PollOnce(&mut checkout2);

        // first poll needed to get into Pool's parked
        poll_once1.await;
        assert_eq!(pool.locked().waiters.get(&key).unwrap().len(), 1);
        poll_once2.await;
        assert_eq!(pool.locked().waiters.get(&key).unwrap().len(), 2);

        // on drop, clean up Pool
        drop(checkout1);
        assert_eq!(pool.locked().waiters.get(&key).unwrap().len(), 1);

        drop(checkout2);
        assert!(!pool.locked().waiters.contains_key(&key));
    }

    #[derive(Debug)]
    struct CanClose {
        #[allow(unused)]
        val: i32,
        closed: bool,
    }

    impl Poolable for CanClose {
        fn is_open(&self) -> bool {
            !self.closed
        }

        fn reserve(self) -> Reservation<Self> {
            Reservation::Unique(self)
        }

        fn can_share(&self) -> bool {
            false
        }
    }

    #[test]
    fn pooled_drop_if_closed_doesnt_reinsert() {
        let pool = pool_no_timer();
        let key = host_key("foo");
        pool.pooled(
            c(key.clone()),
            CanClose {
                val: 57,
                closed: true,
            },
        );

        assert!(!pool.locked().idle.contains_key(&key));
    }
}
