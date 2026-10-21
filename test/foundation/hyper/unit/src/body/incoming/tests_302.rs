#[cfg(test)]
mod tests {
    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    use std::mem;
    #[cfg(all(feature = "nightly", not(miri)))]
    use std::pin::Pin;
    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    use std::task::Poll;

    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    use super::{Body, Incoming, SizeHint};
    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    use super::{DecodedLength, Sender};
    #[cfg(all(feature = "nightly", not(miri)))]
    use bytes::Bytes;
    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    use http_body_util::BodyExt;

    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    #[test]
    fn test_size_of() {
        // These are mostly to help catch *accidentally* increasing
        // the size by too much.

        let body_size = mem::size_of::<Incoming>();
        let body_expected_size = mem::size_of::<u64>() * 5;
        assert!(
            body_size <= body_expected_size,
            "Body size = {} <= {}",
            body_size,
            body_expected_size,
        );

        assert_eq!(
            mem::size_of::<Sender>(),
            mem::size_of::<usize>() * 2,
            "Sender"
        );

        assert_eq!(
            mem::size_of::<Sender>(),
            mem::size_of::<Option<Sender>>(),
            "Option<Sender>"
        );
    }

    #[cfg(all(feature = "nightly", not(miri)))]
    #[bench]
    fn bench_channel_create_and_drop(b: &mut test::Bencher) {
        b.iter(|| {
            let _ = test::black_box(Incoming::new_channel(
                DecodedLength::CHUNKED,
                /* wanter = */ false,
            ));
        });
    }

    #[cfg(all(feature = "nightly", not(miri)))]
    #[bench]
    fn bench_channel_data_handoff(b: &mut test::Bencher) {
        let (mut tx, mut body) =
            Incoming::new_channel(DecodedLength::CHUNKED, /* wanter = */ false);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());

        b.iter(|| {
            assert!(tx.poll_ready(&mut cx).is_ready());
            tx.try_send_data(Bytes::from_static(b"hello world"))
                .unwrap();
            let frame = match Pin::new(&mut body).poll_frame(&mut cx) {
                Poll::Ready(Some(Ok(frame))) => frame,
                unexpected => panic!("unexpected body poll: {unexpected:?}"),
            };
            test::black_box(frame);
        });
    }

    #[cfg(all(feature = "nightly", not(miri)))]
    #[bench]
    fn bench_channel_want_transition(b: &mut test::Bencher) {
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());

        b.iter(|| {
            let (mut tx, mut body) =
                Incoming::new_channel(DecodedLength::CHUNKED, /* wanter = */ true);
            assert!(tx.poll_ready(&mut cx).is_pending());
            assert!(Pin::new(&mut body).poll_frame(&mut cx).is_pending());
            assert!(tx.poll_ready(&mut cx).is_ready());
            let _ = test::black_box((tx, body));
        });
    }

    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    #[test]
    fn size_hint() {
        fn eq(body: Incoming, b: SizeHint, note: &str) {
            let a = body.size_hint();
            assert_eq!(a.lower(), b.lower(), "lower for {:?}", note);
            assert_eq!(a.upper(), b.upper(), "upper for {:?}", note);
        }

        eq(Incoming::empty(), SizeHint::with_exact(0), "empty");

        eq(Incoming::channel().1, SizeHint::new(), "channel");

        eq(
            Incoming::new_channel(DecodedLength::new(4), /*wanter =*/ false).1,
            SizeHint::with_exact(4),
            "channel with length",
        );
    }

    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    #[cfg(not(miri))]
    #[tokio::test]
    async fn channel_abort() {
        let (tx, mut rx) = Incoming::channel();

        tx.abort();

        let err = rx.frame().await.unwrap().unwrap_err();
        assert!(err.is_body_write_aborted(), "{:?}", err);
    }

    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    #[cfg(all(not(miri), feature = "http1"))]
    #[tokio::test]
    async fn channel_abort_when_buffer_is_full() {
        let (mut tx, mut rx) = Incoming::channel();

        tx.try_send_data("chunk 1".into()).expect("send 1");
        // buffer is full, but can still send abort
        tx.abort();

        let chunk1 = rx
            .frame()
            .await
            .expect("item 1")
            .expect("chunk 1")
            .into_data()
            .unwrap();
        assert_eq!(chunk1, "chunk 1");

        let err = rx.frame().await.unwrap().unwrap_err();
        assert!(err.is_body_write_aborted(), "{:?}", err);
    }

    #[cfg(feature = "http1")]
    #[test]
    fn channel_buffers_one() {
        let (mut tx, _rx) = Incoming::channel();

        tx.try_send_data("chunk 1".into()).expect("send 1");

        // buffer is now full
        let chunk2 = tx.try_send_data("chunk 2".into()).expect_err("send 2");
        assert_eq!(chunk2, "chunk 2");
    }

    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    #[cfg(not(miri))]
    #[tokio::test]
    async fn channel_empty() {
        let (_, mut rx) = Incoming::channel();

        assert!(rx.frame().await.is_none());
    }

    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    #[test]
    fn channel_ready() {
        let (mut tx, _rx) = Incoming::new_channel(DecodedLength::CHUNKED, /*wanter = */ false);

        let mut tx_ready = tokio_test::task::spawn(tx.ready());

        assert!(tx_ready.poll().is_ready(), "tx is ready immediately");
    }

    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    #[test]
    fn channel_wanter() {
        let (mut tx, mut rx) =
            Incoming::new_channel(DecodedLength::CHUNKED, /*wanter = */ true);

        let mut tx_ready = tokio_test::task::spawn(tx.ready());
        let mut rx_data = tokio_test::task::spawn(rx.frame());

        assert!(
            tx_ready.poll().is_pending(),
            "tx isn't ready before rx has been polled"
        );

        assert!(rx_data.poll().is_pending(), "poll rx.data");
        assert!(tx_ready.is_woken(), "rx poll wakes tx");

        assert!(
            tx_ready.poll().is_ready(),
            "tx is ready after rx has been polled"
        );
    }

    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    #[test]
    fn channel_notices_closure() {
        let (mut tx, rx) = Incoming::new_channel(DecodedLength::CHUNKED, /*wanter = */ true);

        let mut tx_ready = tokio_test::task::spawn(tx.ready());

        assert!(
            tx_ready.poll().is_pending(),
            "tx isn't ready before rx has been polled"
        );

        drop(rx);
        assert!(tx_ready.is_woken(), "dropping rx wakes tx");

        match &tx_ready.poll() {
            Poll::Ready(Err(e)) if e.is_closed() => (),
            unexpected => panic!("tx poll ready unexpected: {:?}", unexpected),
        }
    }
}
