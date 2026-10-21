#[cfg(all(test, not(crossbeam_loom)))]
mod tests {
    use super::*;
    use crate::{Collector, Owned};
    use crossbeam_utils::thread;
    use std::sync::Barrier;
    use std::vec::Vec;

    impl IsElement<Entry> for Entry {
        fn entry_of(entry: &Entry) -> &Entry {
            entry
        }

        unsafe fn element_of(entry: &Entry) -> &Entry {
            entry
        }

        unsafe fn finalize(entry: &Entry, guard: &Guard) {
            guard.defer_destroy(Shared::from(Self::element_of(entry) as *const _));
        }
    }

    /// Checks whether the list retains inserted elements
    /// and returns them in the correct order.
    #[test]
    fn insert() {
        let collector = Collector::new();
        let handle = collector.register();
        let guard = handle.pin();

        let l: List<Entry> = List::new();

        let e1 = Owned::new(Entry::default()).into_shared(&guard);
        let e2 = Owned::new(Entry::default()).into_shared(&guard);
        let e3 = Owned::new(Entry::default()).into_shared(&guard);

        unsafe {
            l.insert(e1, &guard);
            l.insert(e2, &guard);
            l.insert(e3, &guard);
        }

        let mut iter = l.iter(&guard);
        let maybe_e3 = iter.next();
        assert!(maybe_e3.is_some());
        assert!(maybe_e3.unwrap().unwrap() as *const Entry == e3.as_raw());
        let maybe_e2 = iter.next();
        assert!(maybe_e2.is_some());
        assert!(maybe_e2.unwrap().unwrap() as *const Entry == e2.as_raw());
        let maybe_e1 = iter.next();
        assert!(maybe_e1.is_some());
        assert!(maybe_e1.unwrap().unwrap() as *const Entry == e1.as_raw());
        assert!(iter.next().is_none());

        unsafe {
            e1.as_ref().unwrap().delete(&guard);
            e2.as_ref().unwrap().delete(&guard);
            e3.as_ref().unwrap().delete(&guard);
        }
    }

    /// Checks whether elements can be removed from the list and whether
    /// the correct elements are removed.
    #[test]
    fn delete() {
        let collector = Collector::new();
        let handle = collector.register();
        let guard = handle.pin();

        let l: List<Entry> = List::new();

        let e1 = Owned::new(Entry::default()).into_shared(&guard);
        let e2 = Owned::new(Entry::default()).into_shared(&guard);
        let e3 = Owned::new(Entry::default()).into_shared(&guard);
        unsafe {
            l.insert(e1, &guard);
            l.insert(e2, &guard);
            l.insert(e3, &guard);
            e2.as_ref().unwrap().delete(&guard);
        }

        let mut iter = l.iter(&guard);
        let maybe_e3 = iter.next();
        assert!(maybe_e3.is_some());
        assert!(maybe_e3.unwrap().unwrap() as *const Entry == e3.as_raw());
        let maybe_e1 = iter.next();
        assert!(maybe_e1.is_some());
        assert!(maybe_e1.unwrap().unwrap() as *const Entry == e1.as_raw());
        assert!(iter.next().is_none());

        unsafe {
            e1.as_ref().unwrap().delete(&guard);
            e3.as_ref().unwrap().delete(&guard);
        }

        let mut iter = l.iter(&guard);
        assert!(iter.next().is_none());
    }

    const THREADS: usize = 8;
    const ITERS: usize = 512;

    /// Contends the list on insert and delete operations to make sure they can run concurrently.
    #[test]
    fn insert_delete_multi() {
        let collector = Collector::new();

        let l: List<Entry> = List::new();
        let b = Barrier::new(THREADS);

        thread::scope(|s| {
            for _ in 0..THREADS {
                s.spawn(|_| {
                    b.wait();

                    let handle = collector.register();
                    let guard: Guard = handle.pin();
                    let mut v = Vec::with_capacity(ITERS);

                    for _ in 0..ITERS {
                        let e = Owned::new(Entry::default()).into_shared(&guard);
                        v.push(e);
                        unsafe {
                            l.insert(e, &guard);
                        }
                    }

                    for e in v {
                        unsafe {
                            e.as_ref().unwrap().delete(&guard);
                        }
                    }
                });
            }
        })
        .unwrap();

        let handle = collector.register();
        let guard = handle.pin();

        let mut iter = l.iter(&guard);
        assert!(iter.next().is_none());
    }

    /// Contends the list on iteration to make sure that it can be iterated over concurrently.
    #[test]
    fn iter_multi() {
        let collector = Collector::new();

        let l: List<Entry> = List::new();
        let b = Barrier::new(THREADS);

        thread::scope(|s| {
            for _ in 0..THREADS {
                s.spawn(|_| {
                    b.wait();

                    let handle = collector.register();
                    let guard: Guard = handle.pin();
                    let mut v = Vec::with_capacity(ITERS);

                    for _ in 0..ITERS {
                        let e = Owned::new(Entry::default()).into_shared(&guard);
                        v.push(e);
                        unsafe {
                            l.insert(e, &guard);
                        }
                    }

                    let mut iter = l.iter(&guard);
                    for _ in 0..ITERS {
                        assert!(iter.next().is_some());
                    }

                    for e in v {
                        unsafe {
                            e.as_ref().unwrap().delete(&guard);
                        }
                    }
                });
            }
        })
        .unwrap();

        let handle = collector.register();
        let guard = handle.pin();

        let mut iter = l.iter(&guard);
        assert!(iter.next().is_none());
    }
}
