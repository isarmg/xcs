#[cfg(test)]
mod tests {
    use core::panic::{RefUnwindSafe, UnwindSafe};

    use alloc::{boxed::Box, vec, vec::Vec};

    use super::*;

    #[test]
    fn oibits() {
        fn assert_oitbits<T: Send + Sync + UnwindSafe + RefUnwindSafe>() {}
        assert_oitbits::<Pool<Vec<u32>>>();
        assert_oitbits::<Pool<core::cell::RefCell<Vec<u32>>>>();
        assert_oitbits::<
            Pool<
                Vec<u32>,
                Box<
                    dyn Fn() -> Vec<u32>
                        + Send
                        + Sync
                        + UnwindSafe
                        + RefUnwindSafe,
                >,
            >,
        >();
    }

    // Tests that Pool implements the "single owner" optimization. That is, the
    // thread that first accesses the pool gets its own copy, while all other
    // threads get distinct copies.
    #[cfg(feature = "std")]
    #[test]
    fn thread_owner_optimization() {
        use std::{cell::RefCell, sync::Arc, vec};

        let pool: Arc<Pool<RefCell<Vec<char>>>> =
            Arc::new(Pool::new(|| RefCell::new(vec!['a'])));
        pool.get().borrow_mut().push('x');

        let pool1 = pool.clone();
        let t1 = std::thread::spawn(move || {
            let guard = pool1.get();
            guard.borrow_mut().push('y');
        });

        let pool2 = pool.clone();
        let t2 = std::thread::spawn(move || {
            let guard = pool2.get();
            guard.borrow_mut().push('z');
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // If we didn't implement the single owner optimization, then one of
        // the threads above is likely to have mutated the [a, x] vec that
        // we stuffed in the pool before spawning the threads. But since
        // neither thread was first to access the pool, and because of the
        // optimization, we should be guaranteed that neither thread mutates
        // the special owned pool value.
        //
        // (Technically this is an implementation detail and not a contract of
        // Pool's API.)
        assert_eq!(vec!['a', 'x'], *pool.get().borrow());
    }

    // This tests that if the "owner" of a pool asks for two values, then it
    // gets two distinct values and not the same one. This test failed in the
    // course of developing the pool, which in turn resulted in UB because it
    // permitted getting aliasing &mut borrows to the same place in memory.
    #[test]
    fn thread_owner_distinct() {
        let pool = Pool::new(|| vec!['a']);

        {
            let mut g1 = pool.get();
            let v1 = &mut *g1;
            let mut g2 = pool.get();
            let v2 = &mut *g2;
            v1.push('b');
            v2.push('c');
            assert_eq!(&mut vec!['a', 'b'], v1);
            assert_eq!(&mut vec!['a', 'c'], v2);
        }
        // This isn't technically guaranteed, but we
        // expect to now get the "owned" value (the first
        // call to 'get()' above) now that it's back in
        // the pool.
        assert_eq!(&mut vec!['a', 'b'], &mut *pool.get());
    }

    // This tests that we can share a guard with another thread, mutate the
    // underlying value and everything works. This failed in the course of
    // developing a pool since the pool permitted 'get()' to return the same
    // value to the owner thread, even before the previous value was put back
    // into the pool. This in turn resulted in this test producing a data race.
    #[cfg(feature = "std")]
    #[test]
    fn thread_owner_sync() {
        let pool = Pool::new(|| vec!['a']);
        {
            let mut g1 = pool.get();
            let mut g2 = pool.get();
            std::thread::scope(|s| {
                s.spawn(|| {
                    g1.push('b');
                });
                s.spawn(|| {
                    g2.push('c');
                });
            });

            let v1 = &mut *g1;
            let v2 = &mut *g2;
            assert_eq!(&mut vec!['a', 'b'], v1);
            assert_eq!(&mut vec!['a', 'c'], v2);
        }

        // This isn't technically guaranteed, but we
        // expect to now get the "owned" value (the first
        // call to 'get()' above) now that it's back in
        // the pool.
        assert_eq!(&mut vec!['a', 'b'], &mut *pool.get());
    }

    // This tests that if we move a PoolGuard that is owned by the current
    // thread to another thread and drop it, then the thread owner doesn't
    // change. During development of the pool, this test failed because the
    // PoolGuard assumed it was dropped in the same thread from which it was
    // created, and thus used the current thread's ID as the owner, which could
    // be different than the actual owner of the pool.
    #[cfg(feature = "std")]
    #[test]
    fn thread_owner_send_drop() {
        let pool = Pool::new(|| vec!['a']);
        // Establishes this thread as the owner.
        {
            pool.get().push('b');
        }
        std::thread::scope(|s| {
            // Sanity check that we get the same value back.
            // (Not technically guaranteed.)
            let mut g = pool.get();
            assert_eq!(&vec!['a', 'b'], &*g);
            // Now push it to another thread and drop it.
            s.spawn(move || {
                g.push('c');
            })
            .join()
            .unwrap();
        });
        // Now check that we're still the owner. This is not technically
        // guaranteed by the API, but is true in practice given the thread
        // owner optimization.
        assert_eq!(&vec!['a', 'b', 'c'], &*pool.get());
    }
}
