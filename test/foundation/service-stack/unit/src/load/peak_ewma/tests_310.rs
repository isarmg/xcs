#[cfg(test)]
mod tests {
    use std::{future, time::Duration};
    use tokio::time;
    use tokio_test::{assert_ready, assert_ready_ok, task};

    use super::*;

    struct Svc;
    impl Service<()> for Svc {
        type Response = ();
        type Error = ();
        type Future = future::Ready<Result<(), ()>>;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), ()>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, (): ()) -> Self::Future {
            future::ready(Ok(()))
        }
    }

    /// The default RTT estimate decays, so that new nodes are considered if the
    /// default RTT is too high.
    #[tokio::test]
    async fn default_decay() {
        time::pause();

        let svc = PeakEwma::new(
            Svc,
            Duration::from_millis(10),
            NANOS_PER_MILLI * 1_000.0,
            CompleteOnResponse,
        );
        let Cost(load) = svc.load();
        assert_eq!(load, 10.0 * NANOS_PER_MILLI);

        time::advance(Duration::from_millis(100)).await;
        let Cost(load) = svc.load();
        assert!(9.0 * NANOS_PER_MILLI < load && load < 10.0 * NANOS_PER_MILLI);

        time::advance(Duration::from_millis(100)).await;
        let Cost(load) = svc.load();
        assert!(8.0 * NANOS_PER_MILLI < load && load < 9.0 * NANOS_PER_MILLI);
    }

    // The default RTT estimate decays, so that new nodes are considered if the default RTT is too
    // high.
    #[tokio::test]
    async fn compound_decay() {
        time::pause();

        let mut svc = PeakEwma::new(
            Svc,
            Duration::from_millis(20),
            NANOS_PER_MILLI * 1_000.0,
            CompleteOnResponse,
        );
        assert_eq!(svc.load(), Cost(20.0 * NANOS_PER_MILLI));

        time::advance(Duration::from_millis(100)).await;
        let mut rsp0 = task::spawn(svc.call(()));
        assert!(svc.load() > Cost(20.0 * NANOS_PER_MILLI));

        time::advance(Duration::from_millis(100)).await;
        let mut rsp1 = task::spawn(svc.call(()));
        assert!(svc.load() > Cost(40.0 * NANOS_PER_MILLI));

        time::advance(Duration::from_millis(100)).await;
        let () = assert_ready_ok!(rsp0.poll());
        assert_eq!(svc.load(), Cost(400_000_000.0));

        time::advance(Duration::from_millis(100)).await;
        let () = assert_ready_ok!(rsp1.poll());
        assert_eq!(svc.load(), Cost(200_000_000.0));

        // Check that values decay as time elapses
        time::advance(Duration::from_secs(1)).await;
        assert!(svc.load() < Cost(100_000_000.0));

        time::advance(Duration::from_secs(10)).await;
        assert!(svc.load() < Cost(100_000.0));
    }

    #[test]
    fn nanos() {
        assert_eq!(super::nanos(Duration::new(0, 0)), 0.0);
        assert_eq!(super::nanos(Duration::new(0, 123)), 123.0);
        assert_eq!(super::nanos(Duration::new(1, 23)), 1_000_000_023.0);
        assert_eq!(
            super::nanos(Duration::new(u64::MAX, 999_999_999)),
            18446744074709553000.0
        );
    }
}
