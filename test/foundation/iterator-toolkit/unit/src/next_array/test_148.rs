#[cfg(test)]
mod test {
    use super::ArrayBuilder;

    #[test]
    fn zero_len_take() {
        let mut builder = ArrayBuilder::<(), 0>::new();
        let taken = builder.take();
        assert_eq!(taken, Some([(); 0]));
    }

    #[test]
    #[should_panic]
    fn zero_len_push() {
        let mut builder = ArrayBuilder::<(), 0>::new();
        builder.push(());
    }

    #[test]
    fn push_4() {
        let mut builder = ArrayBuilder::<(), 4>::new();
        assert_eq!(builder.take(), None);

        builder.push(());
        assert_eq!(builder.take(), None);

        builder.push(());
        assert_eq!(builder.take(), None);

        builder.push(());
        assert_eq!(builder.take(), None);

        builder.push(());
        assert_eq!(builder.take(), Some([(); 4]));
    }

    #[test]
    fn tracked_drop() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        use std::sync::atomic::{AtomicU16, Ordering};

        static DROPPED: AtomicU16 = AtomicU16::new(0);

        #[derive(Debug, PartialEq)]
        struct TrackedDrop;

        impl Drop for TrackedDrop {
            fn drop(&mut self) {
                DROPPED.fetch_add(1, Ordering::Relaxed);
            }
        }

        {
            let builder = ArrayBuilder::<TrackedDrop, 0>::new();
            assert_eq!(DROPPED.load(Ordering::Relaxed), 0);
            drop(builder);
            assert_eq!(DROPPED.load(Ordering::Relaxed), 0);
        }

        {
            let mut builder = ArrayBuilder::<TrackedDrop, 2>::new();
            builder.push(TrackedDrop);
            assert_eq!(builder.take(), None);
            assert_eq!(DROPPED.load(Ordering::Relaxed), 0);
            drop(builder);
            assert_eq!(DROPPED.swap(0, Ordering::Relaxed), 1);
        }

        {
            let mut builder = ArrayBuilder::<TrackedDrop, 2>::new();
            builder.push(TrackedDrop);
            builder.push(TrackedDrop);
            assert!(matches!(builder.take(), Some(_)));
            assert_eq!(DROPPED.swap(0, Ordering::Relaxed), 2);
            drop(builder);
            assert_eq!(DROPPED.load(Ordering::Relaxed), 0);
        }

        {
            let mut builder = ArrayBuilder::<TrackedDrop, 2>::new();

            builder.push(TrackedDrop);
            builder.push(TrackedDrop);

            assert!(catch_unwind(AssertUnwindSafe(|| {
                builder.push(TrackedDrop);
            }))
            .is_err());

            assert_eq!(DROPPED.load(Ordering::Relaxed), 1);

            drop(builder);

            assert_eq!(DROPPED.swap(0, Ordering::Relaxed), 3);
        }

        {
            let mut builder = ArrayBuilder::<TrackedDrop, 2>::new();

            builder.push(TrackedDrop);
            builder.push(TrackedDrop);

            assert!(catch_unwind(AssertUnwindSafe(|| {
                builder.push(TrackedDrop);
            }))
            .is_err());

            assert_eq!(DROPPED.load(Ordering::Relaxed), 1);

            assert!(matches!(builder.take(), Some(_)));

            assert_eq!(DROPPED.load(Ordering::Relaxed), 3);

            builder.push(TrackedDrop);
            builder.push(TrackedDrop);

            assert!(matches!(builder.take(), Some(_)));

            assert_eq!(DROPPED.swap(0, Ordering::Relaxed), 5);
        }
    }
}
