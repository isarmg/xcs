#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::RefCell;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering::Relaxed;
    use std::sync::Arc;
    use std::thread;

    fn make_create() -> Arc<dyn Fn() -> usize + Send + Sync> {
        let count = AtomicUsize::new(0);
        Arc::new(move || count.fetch_add(1, Relaxed))
    }

    #[test]
    fn same_thread() {
        let create = make_create();
        let mut tls = ThreadLocal::new();
        assert_eq!(None, tls.get());
        assert_eq!("ThreadLocal { local_data: None }", format!("{:?}", &tls));
        assert_eq!(0, *tls.get_or(|| create()));
        assert_eq!(Some(&0), tls.get());
        assert_eq!(0, *tls.get_or(|| create()));
        assert_eq!(Some(&0), tls.get());
        assert_eq!(0, *tls.get_or(|| create()));
        assert_eq!(Some(&0), tls.get());
        assert_eq!("ThreadLocal { local_data: Some(0) }", format!("{:?}", &tls));
        tls.clear();
        assert_eq!(None, tls.get());
    }

    #[test]
    fn reentrant_get_or() {
        let tls: ThreadLocal<Box<u32>> = ThreadLocal::new();
        let outer = tls.get_or(|| {
            let inner = tls.get_or(|| Box::new(1));
            assert_eq!(**inner, 1);
            Box::new(2)
        });
        // The reentrant call already populated the slot, so the outer
        // value is discarded and the existing value is returned.
        assert_eq!(**outer, 1);
        // Pre-fix, `values` was double-counted and this iterated out of
        // bounds (UB caught by Miri). Post-fix exactly one value is present.
        let collected: Vec<u32> = tls.into_iter().map(|b| *b).collect();
        assert_eq!(collected, vec![1]);
    }

    #[test]
    fn different_thread() {
        let create = make_create();
        let tls = Arc::new(ThreadLocal::new());
        assert_eq!(None, tls.get());
        assert_eq!(0, *tls.get_or(|| create()));
        assert_eq!(Some(&0), tls.get());

        let tls2 = tls.clone();
        let create2 = create.clone();
        thread::spawn(move || {
            assert_eq!(None, tls2.get());
            assert_eq!(1, *tls2.get_or(|| create2()));
            assert_eq!(Some(&1), tls2.get());
        })
        .join()
        .unwrap();

        assert_eq!(Some(&0), tls.get());
        assert_eq!(0, *tls.get_or(|| create()));
    }

    #[test]
    fn iter() {
        let tls = Arc::new(ThreadLocal::new());
        tls.get_or(|| Box::new(1));

        let tls2 = tls.clone();
        thread::spawn(move || {
            tls2.get_or(|| Box::new(2));
            let tls3 = tls2.clone();
            thread::spawn(move || {
                tls3.get_or(|| Box::new(3));
            })
            .join()
            .unwrap();
            drop(tls2);
        })
        .join()
        .unwrap();

        let mut tls = Arc::try_unwrap(tls).unwrap();

        let mut v = tls.iter().map(|x| **x).collect::<Vec<i32>>();
        v.sort_unstable();
        assert_eq!(vec![1, 2, 3], v);

        let mut v = tls.iter_mut().map(|x| **x).collect::<Vec<i32>>();
        v.sort_unstable();
        assert_eq!(vec![1, 2, 3], v);

        let mut v = tls.into_iter().map(|x| *x).collect::<Vec<i32>>();
        v.sort_unstable();
        assert_eq!(vec![1, 2, 3], v);
    }

    #[test]
    fn miri_iter_soundness_check() {
        let tls = Arc::new(ThreadLocal::new());
        let _local = tls.get_or(|| Box::new(1));

        let tls2 = tls.clone();
        let join_1 = thread::spawn(move || {
            let _tls = tls2.get_or(|| Box::new(2));
            let iter = tls2.iter();
            for item in iter {
                println!("{:?}", item);
            }
        });

        let iter = tls.iter();
        for item in iter {
            println!("{:?}", item);
        }

        join_1.join().ok();
    }

    #[test]
    fn test_drop() {
        let local = ThreadLocal::new();
        struct Dropped(Arc<AtomicUsize>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.fetch_add(1, Relaxed);
            }
        }

        let dropped = Arc::new(AtomicUsize::new(0));
        local.get_or(|| Dropped(dropped.clone()));
        assert_eq!(dropped.load(Relaxed), 0);
        drop(local);
        assert_eq!(dropped.load(Relaxed), 1);
    }

    #[test]
    fn test_earlyreturn_buckets() {
        struct Dropped(Arc<AtomicUsize>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.fetch_add(1, Relaxed);
            }
        }
        let dropped = Arc::new(AtomicUsize::new(0));

        // We use a high `id` here to guarantee that a lazily allocated bucket somewhere in the middle is used.
        // Neither iteration nor `Drop` must early-return on `null` buckets that are used for lower `buckets`.
        let thread = Thread::new(1234);
        assert!(thread.bucket > 1);

        let mut local = ThreadLocal::new();
        local.insert(thread, Dropped(dropped.clone()));

        let item = local.iter().next().unwrap();
        assert_eq!(item.0.load(Relaxed), 0);
        let item = local.iter_mut().next().unwrap();
        assert_eq!(item.0.load(Relaxed), 0);
        drop(local);
        assert_eq!(dropped.load(Relaxed), 1);
    }

    #[test]
    fn is_sync() {
        fn foo<T: Sync>() {}
        foo::<ThreadLocal<String>>();
        foo::<ThreadLocal<RefCell<String>>>();
    }
}
