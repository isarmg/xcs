#[cfg(all(test, not(crossbeam_loom)))]
mod tests {
    use std::mem::ManuallyDrop;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::vec::Vec;

    use crossbeam_utils::thread;

    use crate::{Collector, Owned};

    const NUM_THREADS: usize = 8;

    #[test]
    fn pin_reentrant() {
        let collector = Collector::new();
        let handle = collector.register();
        drop(collector);

        assert!(!handle.is_pinned());
        {
            let _guard = &handle.pin();
            assert!(handle.is_pinned());
            {
                let _guard = &handle.pin();
                assert!(handle.is_pinned());
            }
            assert!(handle.is_pinned());
        }
        assert!(!handle.is_pinned());
    }

    #[test]
    fn flush_local_bag() {
        let collector = Collector::new();
        let handle = collector.register();
        drop(collector);

        for _ in 0..100 {
            let guard = &handle.pin();
            unsafe {
                let a = Owned::new(7).into_shared(guard);
                guard.defer_destroy(a);

                assert!(!(*guard.local).bag.with(|b| (*b).is_empty()));

                while !(*guard.local).bag.with(|b| (*b).is_empty()) {
                    guard.flush();
                }
            }
        }
    }

    #[test]
    fn garbage_buffering() {
        let collector = Collector::new();
        let handle = collector.register();
        drop(collector);

        let guard = &handle.pin();
        unsafe {
            for _ in 0..10 {
                let a = Owned::new(7).into_shared(guard);
                guard.defer_destroy(a);
            }
            assert!(!(*guard.local).bag.with(|b| (*b).is_empty()));
        }
    }

    #[test]
    fn pin_holds_advance() {
        #[cfg(miri)]
        const N: usize = 500;
        #[cfg(not(miri))]
        const N: usize = 500_000;

        let collector = Collector::new();

        thread::scope(|scope| {
            for _ in 0..NUM_THREADS {
                scope.spawn(|_| {
                    let handle = collector.register();
                    for _ in 0..N {
                        let guard = &handle.pin();

                        let before = collector.global.epoch.load(Ordering::Relaxed);
                        collector.global.collect(guard);
                        let after = collector.global.epoch.load(Ordering::Relaxed);

                        assert!(after.wrapping_sub(before) <= 2);
                    }
                });
            }
        })
        .unwrap();
    }

    #[cfg(not(crossbeam_sanitize))] // TODO: assertions failed due to `cfg(crossbeam_sanitize)` reduce `internal::MAX_OBJECTS`
    #[test]
    fn incremental() {
        #[cfg(miri)]
        const COUNT: usize = 500;
        #[cfg(not(miri))]
        const COUNT: usize = 100_000;
        static DESTROYS: AtomicUsize = AtomicUsize::new(0);

        let collector = Collector::new();
        let handle = collector.register();

        unsafe {
            let guard = &handle.pin();
            for _ in 0..COUNT {
                let a = Owned::new(7i32).into_shared(guard);
                guard.defer_unchecked(move || {
                    drop(a.into_owned());
                    DESTROYS.fetch_add(1, Ordering::Relaxed);
                });
            }
            guard.flush();
        }

        let mut last = 0;

        while last < COUNT {
            let curr = DESTROYS.load(Ordering::Relaxed);
            assert!(curr - last <= 1024);
            last = curr;

            let guard = &handle.pin();
            collector.global.collect(guard);
        }
        assert!(DESTROYS.load(Ordering::Relaxed) == COUNT);
    }

    #[test]
    fn buffering() {
        const COUNT: usize = 10;
        #[cfg(miri)]
        const N: usize = 500;
        #[cfg(not(miri))]
        const N: usize = 100_000;
        static DESTROYS: AtomicUsize = AtomicUsize::new(0);

        let collector = Collector::new();
        let handle = collector.register();

        unsafe {
            let guard = &handle.pin();
            for _ in 0..COUNT {
                let a = Owned::new(7i32).into_shared(guard);
                guard.defer_unchecked(move || {
                    drop(a.into_owned());
                    DESTROYS.fetch_add(1, Ordering::Relaxed);
                });
            }
        }

        for _ in 0..N {
            collector.global.collect(&handle.pin());
        }
        assert!(DESTROYS.load(Ordering::Relaxed) < COUNT);

        handle.pin().flush();

        while DESTROYS.load(Ordering::Relaxed) < COUNT {
            let guard = &handle.pin();
            collector.global.collect(guard);
        }
        assert_eq!(DESTROYS.load(Ordering::Relaxed), COUNT);
    }

    #[test]
    fn count_drops() {
        #[cfg(miri)]
        const COUNT: usize = 500;
        #[cfg(not(miri))]
        const COUNT: usize = 100_000;
        static DROPS: AtomicUsize = AtomicUsize::new(0);

        struct Elem(#[allow(dead_code)] i32);

        impl Drop for Elem {
            fn drop(&mut self) {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }

        let collector = Collector::new();
        let handle = collector.register();

        unsafe {
            let guard = &handle.pin();

            for _ in 0..COUNT {
                let a = Owned::new(Elem(7i32)).into_shared(guard);
                guard.defer_destroy(a);
            }
            guard.flush();
        }

        while DROPS.load(Ordering::Relaxed) < COUNT {
            let guard = &handle.pin();
            collector.global.collect(guard);
        }
        assert_eq!(DROPS.load(Ordering::Relaxed), COUNT);
    }

    #[test]
    fn count_destroy() {
        #[cfg(miri)]
        const COUNT: usize = 500;
        #[cfg(not(miri))]
        const COUNT: usize = 100_000;
        static DESTROYS: AtomicUsize = AtomicUsize::new(0);

        let collector = Collector::new();
        let handle = collector.register();

        unsafe {
            let guard = &handle.pin();

            for _ in 0..COUNT {
                let a = Owned::new(7i32).into_shared(guard);
                guard.defer_unchecked(move || {
                    drop(a.into_owned());
                    DESTROYS.fetch_add(1, Ordering::Relaxed);
                });
            }
            guard.flush();
        }

        while DESTROYS.load(Ordering::Relaxed) < COUNT {
            let guard = &handle.pin();
            collector.global.collect(guard);
        }
        assert_eq!(DESTROYS.load(Ordering::Relaxed), COUNT);
    }

    #[test]
    fn drop_array() {
        const COUNT: usize = 700;
        static DROPS: AtomicUsize = AtomicUsize::new(0);

        struct Elem(#[allow(dead_code)] i32);

        impl Drop for Elem {
            fn drop(&mut self) {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }

        let collector = Collector::new();
        let handle = collector.register();

        let mut guard = handle.pin();

        let mut v = Vec::with_capacity(COUNT);
        for i in 0..COUNT {
            v.push(Elem(i as i32));
        }

        {
            let a = Owned::new(v).into_shared(&guard);
            unsafe {
                guard.defer_destroy(a);
            }
            guard.flush();
        }

        while DROPS.load(Ordering::Relaxed) < COUNT {
            guard.repin();
            collector.global.collect(&guard);
        }
        assert_eq!(DROPS.load(Ordering::Relaxed), COUNT);
    }

    #[test]
    fn destroy_array() {
        #[cfg(miri)]
        const COUNT: usize = 500;
        #[cfg(not(miri))]
        const COUNT: usize = 100_000;
        static DESTROYS: AtomicUsize = AtomicUsize::new(0);

        let collector = Collector::new();
        let handle = collector.register();

        unsafe {
            let guard = &handle.pin();

            let mut v = Vec::with_capacity(COUNT);
            for i in 0..COUNT {
                v.push(i as i32);
            }

            let len = v.len();
            let cap = v.capacity();
            let ptr = ManuallyDrop::new(v).as_mut_ptr();
            guard.defer_unchecked(move || {
                drop(Vec::from_raw_parts(ptr, len, cap));
                DESTROYS.fetch_add(len, Ordering::Relaxed);
            });
            guard.flush();
        }

        while DESTROYS.load(Ordering::Relaxed) < COUNT {
            let guard = &handle.pin();
            collector.global.collect(guard);
        }
        assert_eq!(DESTROYS.load(Ordering::Relaxed), COUNT);
    }

    #[test]
    fn stress() {
        const THREADS: usize = 8;
        #[cfg(miri)]
        const COUNT: usize = 500;
        #[cfg(not(miri))]
        const COUNT: usize = 100_000;
        static DROPS: AtomicUsize = AtomicUsize::new(0);

        struct Elem(#[allow(dead_code)] i32);

        impl Drop for Elem {
            fn drop(&mut self) {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }

        let collector = Collector::new();

        thread::scope(|scope| {
            for _ in 0..THREADS {
                scope.spawn(|_| {
                    let handle = collector.register();
                    for _ in 0..COUNT {
                        let guard = &handle.pin();
                        unsafe {
                            let a = Owned::new(Elem(7i32)).into_shared(guard);
                            guard.defer_destroy(a);
                        }
                    }
                });
            }
        })
        .unwrap();

        let handle = collector.register();
        while DROPS.load(Ordering::Relaxed) < COUNT * THREADS {
            let guard = &handle.pin();
            collector.global.collect(guard);
        }
        assert_eq!(DROPS.load(Ordering::Relaxed), COUNT * THREADS);
    }
}
