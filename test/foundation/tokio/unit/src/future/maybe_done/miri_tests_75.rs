#[cfg(test)]
mod miri_tests {
    use super::maybe_done;

    use std::{
        future::Future,
        pin::Pin,
        sync::Arc,
        task::{Context, Poll, Wake},
    };

    struct ThingAdder<'a> {
        thing: &'a mut String,
    }

    impl Future for ThingAdder<'_> {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            unsafe {
                *self.get_unchecked_mut().thing += ", world";
            }
            Poll::Pending
        }
    }

    #[test]
    fn maybe_done_miri() {
        let mut thing = "hello".to_owned();

        // The async block is necessary to trigger the miri failure.
        #[allow(clippy::redundant_async_block)]
        let fut = async move { ThingAdder { thing: &mut thing }.await };

        let mut fut = maybe_done(fut);
        let mut fut = unsafe { Pin::new_unchecked(&mut fut) };

        let waker = Arc::new(DummyWaker).into();
        let mut ctx = Context::from_waker(&waker);
        assert_eq!(fut.as_mut().poll(&mut ctx), Poll::Pending);
        assert_eq!(fut.as_mut().poll(&mut ctx), Poll::Pending);
    }

    struct DummyWaker;

    impl Wake for DummyWaker {
        fn wake(self: Arc<Self>) {}
    }
}
