#[cfg(test)]
mod test_par_map {
    use core::hash::{Hash, Hasher};
    use core::sync::atomic::{AtomicUsize, Ordering};
    use stdalloc::vec::Vec;

    use rayon::prelude::*;

    use crate::HashMap;

    struct Droppable<'a> {
        k: usize,
        counter: &'a AtomicUsize,
    }

    impl Droppable<'_> {
        fn new(k: usize, counter: &AtomicUsize) -> Droppable<'_> {
            counter.fetch_add(1, Ordering::Relaxed);

            Droppable { k, counter }
        }
    }

    impl Drop for Droppable<'_> {
        fn drop(&mut self) {
            self.counter.fetch_sub(1, Ordering::Relaxed);
        }
    }

    impl Clone for Droppable<'_> {
        fn clone(&self) -> Self {
            Droppable::new(self.k, self.counter)
        }
    }

    impl Hash for Droppable<'_> {
        fn hash<H>(&self, state: &mut H)
        where
            H: Hasher,
        {
            self.k.hash(state);
        }
    }

    impl PartialEq for Droppable<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.k == other.k
        }
    }

    impl Eq for Droppable<'_> {}

    #[test]
    fn test_into_iter_drops() {
        let key = AtomicUsize::new(0);
        let value = AtomicUsize::new(0);

        let hm = {
            let mut hm = HashMap::new();

            assert_eq!(key.load(Ordering::Relaxed), 0);
            assert_eq!(value.load(Ordering::Relaxed), 0);

            for i in 0..100 {
                let d1 = Droppable::new(i, &key);
                let d2 = Droppable::new(i + 100, &value);
                hm.insert(d1, d2);
            }

            assert_eq!(key.load(Ordering::Relaxed), 100);
            assert_eq!(value.load(Ordering::Relaxed), 100);

            hm
        };

        // By the way, ensure that cloning doesn't screw up the dropping.
        drop(hm.clone());

        assert_eq!(key.load(Ordering::Relaxed), 100);
        assert_eq!(value.load(Ordering::Relaxed), 100);

        // Ensure that dropping the iterator does not leak anything.
        drop(hm.clone().into_par_iter());

        {
            assert_eq!(key.load(Ordering::Relaxed), 100);
            assert_eq!(value.load(Ordering::Relaxed), 100);

            // retain only half
            let _v: Vec<_> = hm.into_par_iter().filter(|(key, _)| key.k < 50).collect();

            assert_eq!(key.load(Ordering::Relaxed), 50);
            assert_eq!(value.load(Ordering::Relaxed), 50);
        };

        assert_eq!(key.load(Ordering::Relaxed), 0);
        assert_eq!(value.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_drain_drops() {
        let key = AtomicUsize::new(0);
        let value = AtomicUsize::new(0);

        let mut hm = {
            let mut hm = HashMap::new();

            assert_eq!(key.load(Ordering::Relaxed), 0);
            assert_eq!(value.load(Ordering::Relaxed), 0);

            for i in 0..100 {
                let d1 = Droppable::new(i, &key);
                let d2 = Droppable::new(i + 100, &value);
                hm.insert(d1, d2);
            }

            assert_eq!(key.load(Ordering::Relaxed), 100);
            assert_eq!(value.load(Ordering::Relaxed), 100);

            hm
        };

        // By the way, ensure that cloning doesn't screw up the dropping.
        drop(hm.clone());

        assert_eq!(key.load(Ordering::Relaxed), 100);
        assert_eq!(value.load(Ordering::Relaxed), 100);

        // Ensure that dropping the drain iterator does not leak anything.
        drop(hm.clone().par_drain());

        {
            assert_eq!(key.load(Ordering::Relaxed), 100);
            assert_eq!(value.load(Ordering::Relaxed), 100);

            // retain only half
            let _v: Vec<_> = hm.drain().filter(|(key, _)| key.k < 50).collect();
            assert!(hm.is_empty());

            assert_eq!(key.load(Ordering::Relaxed), 50);
            assert_eq!(value.load(Ordering::Relaxed), 50);
        };

        assert_eq!(key.load(Ordering::Relaxed), 0);
        assert_eq!(value.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_empty_iter() {
        let mut m: HashMap<isize, bool> = HashMap::new();
        assert_eq!(m.par_drain().count(), 0);
        assert_eq!(m.par_keys().count(), 0);
        assert_eq!(m.par_values().count(), 0);
        assert_eq!(m.par_values_mut().count(), 0);
        assert_eq!(m.par_iter().count(), 0);
        assert_eq!(m.par_iter_mut().count(), 0);
        assert_eq!(m.len(), 0);
        assert!(m.is_empty());
        assert_eq!(m.into_par_iter().count(), 0);
    }

    #[test]
    fn test_iterate() {
        let mut m = HashMap::with_capacity(4);
        for i in 0..32 {
            assert!(m.insert(i, i * 2).is_none());
        }
        assert_eq!(m.len(), 32);

        let observed = AtomicUsize::new(0);

        m.par_iter().for_each(|(k, v)| {
            assert_eq!(*v, *k * 2);
            observed.fetch_or(1 << *k, Ordering::Relaxed);
        });
        assert_eq!(observed.into_inner(), 0xFFFF_FFFF);
    }

    #[test]
    fn test_keys() {
        let vec = vec![(1, 'a'), (2, 'b'), (3, 'c')];
        let map: HashMap<_, _> = vec.into_par_iter().collect();
        let keys: Vec<_> = map.par_keys().cloned().collect();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&1));
        assert!(keys.contains(&2));
        assert!(keys.contains(&3));
    }

    #[test]
    fn test_values() {
        let vec = vec![(1, 'a'), (2, 'b'), (3, 'c')];
        let map: HashMap<_, _> = vec.into_par_iter().collect();
        let values: Vec<_> = map.par_values().cloned().collect();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&'a'));
        assert!(values.contains(&'b'));
        assert!(values.contains(&'c'));
    }

    #[test]
    fn test_values_mut() {
        let vec = vec![(1, 1), (2, 2), (3, 3)];
        let mut map: HashMap<_, _> = vec.into_par_iter().collect();
        map.par_values_mut().for_each(|value| *value *= 2);
        let values: Vec<_> = map.par_values().cloned().collect();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&2));
        assert!(values.contains(&4));
        assert!(values.contains(&6));
    }

    #[test]
    fn test_eq() {
        let mut m1 = HashMap::new();
        m1.insert(1, 2);
        m1.insert(2, 3);
        m1.insert(3, 4);

        let mut m2 = HashMap::new();
        m2.insert(1, 2);
        m2.insert(2, 3);

        assert!(!m1.par_eq(&m2));

        m2.insert(3, 4);

        assert!(m1.par_eq(&m2));
    }

    #[test]
    fn test_from_iter() {
        let xs = [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)];

        let map: HashMap<_, _> = xs.par_iter().cloned().collect();

        for &(k, v) in &xs {
            assert_eq!(map.get(&k), Some(&v));
        }
    }

    #[test]
    fn test_extend_ref() {
        let mut a = HashMap::new();
        a.insert(1, "one");
        let mut b = HashMap::new();
        b.insert(2, "two");
        b.insert(3, "three");

        a.par_extend(&b);

        assert_eq!(a.len(), 3);
        assert_eq!(a[&1], "one");
        assert_eq!(a[&2], "two");
        assert_eq!(a[&3], "three");
    }
}
