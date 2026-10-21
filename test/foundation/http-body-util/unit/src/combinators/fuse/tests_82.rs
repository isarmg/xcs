#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::collections::VecDeque;

    /// A value returned by a call to [`Body::poll_frame()`].
    type PollFrame = Poll<Option<Result<Frame<Bytes>, Error>>>;

    type Error = &'static str;

    struct Mock<'count> {
        poll_count: &'count mut u8,
        polls: VecDeque<PollFrame>,
    }

    #[test]
    fn empty_never_polls() {
        let mut count = 0_u8;
        let empty = Mock::new(&mut count, []);
        debug_assert!(empty.is_end_stream());
        let fused = Fuse::new(empty);
        assert!(fused.inner.is_none());
        drop(fused);
        assert_eq!(count, 0);
    }

    #[test]
    fn stops_polling_after_none() {
        let mut count = 0_u8;
        let empty = Mock::new(&mut count, [Poll::Ready(None)]);
        debug_assert!(!empty.is_end_stream());
        let mut fused = Fuse::new(empty);
        assert!(fused.inner.is_some());

        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        match Pin::new(&mut fused).poll_frame(&mut cx) {
            Poll::Ready(None) => {}
            other => panic!("unexpected poll outcome: {:?}", other),
        }

        assert!(fused.inner.is_none());
        match Pin::new(&mut fused).poll_frame(&mut cx) {
            Poll::Ready(None) => {}
            other => panic!("unexpected poll outcome: {:?}", other),
        }

        drop(fused);
        assert_eq!(count, 1);
    }

    #[test]
    fn stops_polling_after_some_eos() {
        let mut count = 0_u8;
        let body = Mock::new(
            &mut count,
            [Poll::Ready(Some(Ok(Frame::data(Bytes::from_static(
                b"hello",
            )))))],
        );
        debug_assert!(!body.is_end_stream());
        let mut fused = Fuse::new(body);
        assert!(fused.inner.is_some());

        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        match Pin::new(&mut fused).poll_frame(&mut cx) {
            Poll::Ready(Some(Ok(bytes))) => assert_eq!(bytes.into_data().expect("data"), "hello"),
            other => panic!("unexpected poll outcome: {:?}", other),
        }

        assert!(fused.inner.is_none());
        match Pin::new(&mut fused).poll_frame(&mut cx) {
            Poll::Ready(None) => {}
            other => panic!("unexpected poll outcome: {:?}", other),
        }

        drop(fused);
        assert_eq!(count, 1);
    }

    #[test]
    fn stops_polling_after_some_error() {
        let mut count = 0_u8;
        let body = Mock::new(
            &mut count,
            [
                Poll::Ready(Some(Ok(Frame::data(Bytes::from_static(b"hello"))))),
                Poll::Ready(Some(Err("oh no"))),
                Poll::Ready(Some(Ok(Frame::data(Bytes::from_static(b"world"))))),
            ],
        );
        debug_assert!(!body.is_end_stream());
        let mut fused = Fuse::new(body);
        assert!(fused.inner.is_some());

        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        match Pin::new(&mut fused).poll_frame(&mut cx) {
            Poll::Ready(Some(Ok(bytes))) => assert_eq!(bytes.into_data().expect("data"), "hello"),
            other => panic!("unexpected poll outcome: {:?}", other),
        }

        assert!(fused.inner.is_some());
        match Pin::new(&mut fused).poll_frame(&mut cx) {
            Poll::Ready(Some(Err("oh no"))) => {}
            other => panic!("unexpected poll outcome: {:?}", other),
        }

        assert!(fused.inner.is_none());
        match Pin::new(&mut fused).poll_frame(&mut cx) {
            Poll::Ready(None) => {}
            other => panic!("unexpected poll outcome: {:?}", other),
        }

        drop(fused);
        assert_eq!(count, 2);
    }

    // === impl Mock ===

    impl<'count> Mock<'count> {
        fn new(poll_count: &'count mut u8, polls: impl IntoIterator<Item = PollFrame>) -> Self {
            Self {
                poll_count,
                polls: polls.into_iter().collect(),
            }
        }
    }

    impl Body for Mock<'_> {
        type Data = Bytes;
        type Error = &'static str;

        fn poll_frame(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
            let Self { poll_count, polls } = self.get_mut();
            **poll_count = poll_count.saturating_add(1);
            polls.pop_front().unwrap_or(Poll::Ready(None))
        }

        fn is_end_stream(&self) -> bool {
            self.polls.is_empty()
        }
    }
}
