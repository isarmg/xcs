#[cfg(test)]
mod tests {
    use std::prelude::v1::*;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::thread;

    type RwLock<T> = super::RwLock<T>;

    #[derive(Eq, PartialEq, Debug)]
    struct NonCopy(i32);

    #[test]
    fn smoke() {
        let l = RwLock::new(());
        drop(l.read());
        drop(l.write());
        drop((l.read(), l.read()));
        drop(l.write());
    }

    // TODO: needs RNG
    //#[test]
    //fn frob() {
    //    static R: RwLock = RwLock::new();
    //    const N: usize = 10;
    //    const M: usize = 1000;
    //
    //    let (tx, rx) = channel::<()>();
    //    for _ in 0..N {
    //        let tx = tx.clone();
    //        thread::spawn(move|| {
    //            let mut rng = rand::thread_rng();
    //            for _ in 0..M {
    //                if rng.gen_weighted_bool(N) {
    //                    drop(R.write());
    //                } else {
    //                    drop(R.read());
    //                }
    //            }
    //            drop(tx);
    //        });
    //    }
    //    drop(tx);
    //    let _ = rx.recv();
    //    unsafe { R.destroy(); }
    //}

    #[test]
    fn test_rw_arc() {
        let arc = Arc::new(RwLock::new(0));
        let arc2 = arc.clone();
        let (tx, rx) = channel();

        let t = thread::spawn(move || {
            let mut lock = arc2.write();
            for _ in 0..10 {
                let tmp = *lock;
                *lock = -1;
                thread::yield_now();
                *lock = tmp + 1;
            }
            tx.send(()).unwrap();
        });

        // Readers try to catch the writer in the act
        let mut children = Vec::new();
        for _ in 0..5 {
            let arc3 = arc.clone();
            children.push(thread::spawn(move || {
                let lock = arc3.read();
                assert!(*lock >= 0);
            }));
        }

        // Wait for children to pass their asserts
        for r in children {
            assert!(r.join().is_ok());
        }

        // Wait for writer to finish
        rx.recv().unwrap();
        let lock = arc.read();
        assert_eq!(*lock, 10);

        assert!(t.join().is_ok());
    }

    #[test]
    fn test_rw_access_in_unwind() {
        let arc = Arc::new(RwLock::new(1));
        let arc2 = arc.clone();
        let _ = thread::spawn(move || -> () {
            struct Unwinder {
                i: Arc<RwLock<isize>>,
            }
            impl Drop for Unwinder {
                fn drop(&mut self) {
                    let mut lock = self.i.write();
                    *lock += 1;
                }
            }
            let _u = Unwinder { i: arc2 };
            panic!();
        })
        .join();
        let lock = arc.read();
        assert_eq!(*lock, 2);
    }

    #[test]
    fn test_rwlock_unsized() {
        let rw: &RwLock<[i32]> = &RwLock::new([1, 2, 3]);
        {
            let b = &mut *rw.write();
            b[0] = 4;
            b[2] = 5;
        }
        let comp: &[i32] = &[4, 2, 5];
        assert_eq!(&*rw.read(), comp);
    }

    #[test]
    fn test_rwlock_try_write() {
        use std::mem::drop;

        let lock = RwLock::new(0isize);
        let read_guard = lock.read();

        let write_result = lock.try_write();
        match write_result {
            None => (),
            Some(_) => assert!(
                false,
                "try_write should not succeed while read_guard is in scope"
            ),
        }

        drop(read_guard);
    }

    #[test]
    fn test_rw_try_read() {
        let m = RwLock::new(0);
        ::std::mem::forget(m.write());
        assert!(m.try_read().is_none());
    }

    #[test]
    fn test_into_inner() {
        let m = RwLock::new(NonCopy(10));
        assert_eq!(m.into_inner(), NonCopy(10));
    }

    #[test]
    fn test_into_inner_drop() {
        struct Foo(Arc<AtomicUsize>);
        impl Drop for Foo {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let num_drops = Arc::new(AtomicUsize::new(0));
        let m = RwLock::new(Foo(num_drops.clone()));
        assert_eq!(num_drops.load(Ordering::SeqCst), 0);
        {
            let _inner = m.into_inner();
            assert_eq!(num_drops.load(Ordering::SeqCst), 0);
        }
        assert_eq!(num_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_force_read_decrement() {
        let m = RwLock::new(());
        ::std::mem::forget(m.read());
        ::std::mem::forget(m.read());
        ::std::mem::forget(m.read());
        assert!(m.try_write().is_none());
        unsafe {
            m.force_read_decrement();
            m.force_read_decrement();
        }
        assert!(m.try_write().is_none());
        unsafe {
            m.force_read_decrement();
        }
        assert!(m.try_write().is_some());
    }

    #[test]
    fn test_force_write_unlock() {
        let m = RwLock::new(());
        ::std::mem::forget(m.write());
        assert!(m.try_read().is_none());
        unsafe {
            m.force_write_unlock();
        }
        assert!(m.try_read().is_some());
    }

    #[test]
    fn test_upgrade_downgrade() {
        let m = RwLock::new(());
        {
            let _r = m.read();
            let upg = m.try_upgradeable_read().unwrap();
            assert!(m.try_read().is_none());
            assert!(m.try_write().is_none());
            assert!(upg.try_upgrade().is_err());
        }
        {
            let w = m.write();
            assert!(m.try_upgradeable_read().is_none());
            let _r = w.downgrade();
            assert!(m.try_upgradeable_read().is_some());
            assert!(m.try_read().is_some());
            assert!(m.try_write().is_none());
        }
        {
            let _u = m.upgradeable_read();
            assert!(m.try_upgradeable_read().is_none());
        }

        assert!(m.try_upgradeable_read().unwrap().try_upgrade().is_ok());
    }
}
