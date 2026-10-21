#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        task::{Wake, Waker},
    };

    use super::*;

    async fn recv(rx: &mut Receiver) -> Option<Result<Bytes, crate::Error>> {
        futures::future::poll_fn(|cx| rx.poll_next(cx)).await
    }

    #[tokio::test]
    async fn error_queued_behind_accepted_data() {
        let (mut tx, mut rx) = channel(false);
        tx.try_send_data(Bytes::from_static(b"data")).unwrap();
        tx.send_error(crate::Error::new_incomplete());
        drop(tx);

        assert_eq!(recv(&mut rx).await.unwrap().unwrap(), "data");
        assert!(recv(&mut rx).await.unwrap().is_err());
        assert!(recv(&mut rx).await.is_none());
    }

    #[tokio::test]
    async fn trailers_follow_data_close() {
        let (mut tx, mut rx) = channel(false);
        let mut trailers = HeaderMap::new();
        trailers.insert("x-trailer", "value".parse().unwrap());
        tx.try_send_trailers(trailers).unwrap();
        drop(tx);

        assert!(recv(&mut rx).await.is_none());
        assert_eq!(rx.take_trailers().unwrap()["x-trailer"], "value");
    }

    #[test]
    fn drop_wakes_only_an_open_peer() {
        struct Count(AtomicUsize);

        impl Wake for Count {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }

        let count = Arc::new(Count(AtomicUsize::new(0)));
        let waker = Waker::from(count.clone());
        let mut cx = Context::from_waker(&waker);
        let wakes = || count.0.load(Ordering::Relaxed);

        // A receiver dropped after the sender does not wake the sender's stale waker.
        let (mut tx, rx) = channel(true);
        assert!(tx.poll_ready(&mut cx).is_pending());
        drop(tx);
        drop(rx);
        assert_eq!(wakes(), 0);

        // Nor does a sender dropped after the receiver.
        let (tx, mut rx) = channel(false);
        assert!(rx.poll_next(&mut cx).is_pending());
        drop(rx);
        drop(tx);
        assert_eq!(wakes(), 0);

        // An open peer is still woken to observe the close.
        let (mut tx, rx) = channel(true);
        assert!(tx.poll_ready(&mut cx).is_pending());
        drop(rx);
        assert_eq!(wakes(), 1);
        let (tx, mut rx) = channel(false);
        assert!(rx.poll_next(&mut cx).is_pending());
        drop(tx);
        assert_eq!(wakes(), 2);
    }
}
