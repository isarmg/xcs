#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::StreamExt;
    use core::marker::PhantomData;

    struct NopStream<Item> {
        phantom: PhantomData<Item>,
    }

    impl<Item> Stream for NopStream<Item> {
        type Item = Item;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            todo!()
        }
    }

    impl<Item> Sink<Item> for NopStream<Item> {
        type Error = ();

        fn poll_ready(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            todo!()
        }

        fn start_send(self: Pin<&mut Self>, _item: Item) -> Result<(), Self::Error> {
            todo!()
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            todo!()
        }

        fn poll_close(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            todo!()
        }
    }

    #[test]
    fn test_pairing() {
        let s1 = NopStream::<()> { phantom: PhantomData };
        let (sink1, stream1) = s1.split();
        assert!(sink1.is_pair_of(&stream1));
        assert!(stream1.is_pair_of(&sink1));

        let s2 = NopStream::<()> { phantom: PhantomData };
        let (sink2, stream2) = s2.split();
        assert!(sink2.is_pair_of(&stream2));
        assert!(stream2.is_pair_of(&sink2));

        assert!(!sink1.is_pair_of(&stream2));
        assert!(!stream1.is_pair_of(&sink2));
        assert!(!sink2.is_pair_of(&stream1));
        assert!(!stream2.is_pair_of(&sink1));
    }
}
