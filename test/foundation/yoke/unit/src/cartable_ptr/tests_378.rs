#[cfg(test)]
mod tests {
    use super::*;
    use crate::Yoke;

    const SAMPLE_BYTES: &[u8] = b"abCDEfg";
    const W: usize = size_of::<usize>();

    #[test]
    fn test_sizes() {
        assert_eq!(W * 4, size_of::<Yoke<[usize; 3], &&[u8]>>());
        assert_eq!(W * 4, size_of::<Yoke<[usize; 3], Option<&&[u8]>>>());
        assert_eq!(
            W * 4,
            size_of::<Yoke<[usize; 3], CartableOptionPointer<&&[u8]>>>()
        );

        assert_eq!(W * 4, size_of::<Option<Yoke<[usize; 3], &&[u8]>>>());
        assert_eq!(W * 5, size_of::<Option<Yoke<[usize; 3], Option<&&[u8]>>>>());
        assert_eq!(
            W * 4,
            size_of::<Option<Yoke<[usize; 3], CartableOptionPointer<&&[u8]>>>>()
        );
    }

    #[test]
    fn test_new_sentinel() {
        let start = DROP_INVOCATIONS.with(Cell::get);
        {
            let _ = CartableOptionPointer::<Rc<&[u8]>>::none();
        }
        assert_eq!(start, DROP_INVOCATIONS.with(Cell::get));
        {
            let _ = CartableOptionPointer::<Rc<&[u8]>>::none();
        }
        assert_eq!(start, DROP_INVOCATIONS.with(Cell::get));
    }

    #[test]
    fn test_new_rc() {
        let start = DROP_INVOCATIONS.with(Cell::get);
        {
            let _ = CartableOptionPointer::<Rc<&[u8]>>::from_cartable(SAMPLE_BYTES.into());
        }
        assert_eq!(start + 1, DROP_INVOCATIONS.with(Cell::get));
    }

    #[test]
    fn test_rc_clone() {
        let start = DROP_INVOCATIONS.with(Cell::get);
        {
            let x = CartableOptionPointer::<Rc<&[u8]>>::from_cartable(SAMPLE_BYTES.into());
            assert_eq!(start, DROP_INVOCATIONS.with(Cell::get));
            {
                let _ = x.clone();
            }
            assert_eq!(start, DROP_INVOCATIONS.with(Cell::get));
            {
                let _ = x.clone();
                let _ = x.clone();
                let _ = x.clone();
            }
            assert_eq!(start, DROP_INVOCATIONS.with(Cell::get));
        }
        assert_eq!(start + 1, DROP_INVOCATIONS.with(Cell::get));
    }

    // Miri test for CartableOptionPointer Send/Sync soundness.
    //
    // Verifies that a CartableOptionPointer wrapping a Send + !Sync cart can be
    // safely sent to another thread and dropped, which compiles under the corrected
    // bounds but would fail under the old (backwards) bounds.
    #[test]
    fn test_send_drops_on_another_thread() {
        use std::marker::PhantomData;

        // ThreadDrop is Send but !Sync
        struct ThreadDrop {
            _not_sync: PhantomData<Cell<i32>>,
        }

        let yoke = Yoke::<usize, Box<ThreadDrop>>::attach_to_cart(
            Box::new(ThreadDrop {
                _not_sync: PhantomData,
            }),
            |_| 0,
        )
        .wrap_cart_in_option()
        .convert_cart_into_option_pointer();

        // This requires CartableOptionPointer to be Send.
        // It is Send because Box<ThreadDrop> is Send.
        // It would NOT be Send under the old bounds because Box<ThreadDrop> is !Sync.
        std::thread::spawn(move || drop(yoke)).join().unwrap();
    }
}
