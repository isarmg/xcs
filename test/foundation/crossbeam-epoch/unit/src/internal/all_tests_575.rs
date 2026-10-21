#[cfg(all(test, not(crossbeam_loom)))]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    #[test]
    fn check_defer() {
        static FLAG: AtomicUsize = AtomicUsize::new(0);
        fn set() {
            FLAG.store(42, Ordering::Relaxed);
        }

        let d = Deferred::new(set);
        assert_eq!(FLAG.load(Ordering::Relaxed), 0);
        d.call();
        assert_eq!(FLAG.load(Ordering::Relaxed), 42);
    }

    #[test]
    fn check_bag() {
        static FLAG: AtomicUsize = AtomicUsize::new(0);
        fn incr() {
            FLAG.fetch_add(1, Ordering::Relaxed);
        }

        let mut bag = Bag::new();
        assert!(bag.is_empty());

        for _ in 0..MAX_OBJECTS {
            assert!(unsafe { bag.try_push(Deferred::new(incr)).is_ok() });
            assert!(!bag.is_empty());
            assert_eq!(FLAG.load(Ordering::Relaxed), 0);
        }

        let result = unsafe { bag.try_push(Deferred::new(incr)) };
        assert!(result.is_err());
        assert!(!bag.is_empty());
        assert_eq!(FLAG.load(Ordering::Relaxed), 0);

        drop(bag);
        assert_eq!(FLAG.load(Ordering::Relaxed), MAX_OBJECTS);
    }
}
