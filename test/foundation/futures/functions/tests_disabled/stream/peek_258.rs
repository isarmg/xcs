#[test]
fn peek() {
    struct Peek {
        inner: Peekable<Box<Stream<Item = i32, Error = u32> + Send>>,
    }

    impl Future for Peek {
        type Item = ();
        type Error = u32;

        fn poll(&mut self, cx: &mut Context<'_>) -> Poll<(), u32> {
            {
                let res = ready!(self.inner.peek(cx))?;
                assert_eq!(res, Some(&1));
            }
            assert_eq!(self.inner.peek(cx).unwrap(), Some(&1).into());
            assert_eq!(self.inner.poll_next(cx).unwrap(), Some(1).into());
            Ok(Poll::Ready(()))
        }
    }

    block_on(Peek { inner: list().peekable() }).unwrap()
}
