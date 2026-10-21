#[cfg(test)]
mod test_map {
    use super::HashMap;
    use super::RawEntryMut;

    #[test]
    fn test_raw_occupied_entry_replace_entry_with() {
        let mut a = HashMap::new();

        let key = "a key";
        let value = "an initial value";
        let new_value = "a new value";

        let entry = a
            .raw_entry_mut()
            .from_key(&key)
            .insert(key, value)
            .replace_entry_with(|k, v| {
                assert_eq!(k, &key);
                assert_eq!(v, value);
                Some(new_value)
            });

        match entry {
            RawEntryMut::Occupied(e) => {
                assert_eq!(e.key(), &key);
                assert_eq!(e.get(), &new_value);
            }
            RawEntryMut::Vacant(_) => panic!(),
        }

        assert_eq!(a[key], new_value);
        assert_eq!(a.len(), 1);

        let entry = match a.raw_entry_mut().from_key(&key) {
            RawEntryMut::Occupied(e) => e.replace_entry_with(|k, v| {
                assert_eq!(k, &key);
                assert_eq!(v, new_value);
                None
            }),
            RawEntryMut::Vacant(_) => panic!(),
        };

        match entry {
            RawEntryMut::Vacant(_) => {}
            RawEntryMut::Occupied(_) => panic!(),
        }

        assert!(!a.contains_key(key));
        assert_eq!(a.len(), 0);
    }

    #[test]
    fn test_raw_entry_and_replace_entry_with() {
        let mut a = HashMap::new();

        let key = "a key";
        let value = "an initial value";
        let new_value = "a new value";

        let entry = a
            .raw_entry_mut()
            .from_key(&key)
            .and_replace_entry_with(|_, _| panic!());

        match entry {
            RawEntryMut::Vacant(_) => {}
            RawEntryMut::Occupied(_) => panic!(),
        }

        a.insert(key, value);

        let entry = a
            .raw_entry_mut()
            .from_key(&key)
            .and_replace_entry_with(|k, v| {
                assert_eq!(k, &key);
                assert_eq!(v, value);
                Some(new_value)
            });

        match entry {
            RawEntryMut::Occupied(e) => {
                assert_eq!(e.key(), &key);
                assert_eq!(e.get(), &new_value);
            }
            RawEntryMut::Vacant(_) => panic!(),
        }

        assert_eq!(a[key], new_value);
        assert_eq!(a.len(), 1);

        let entry = a
            .raw_entry_mut()
            .from_key(&key)
            .and_replace_entry_with(|k, v| {
                assert_eq!(k, &key);
                assert_eq!(v, new_value);
                None
            });

        match entry {
            RawEntryMut::Vacant(_) => {}
            RawEntryMut::Occupied(_) => panic!(),
        }

        assert!(!a.contains_key(key));
        assert_eq!(a.len(), 0);
    }

    #[test]
    fn test_raw_entry() {
        use super::RawEntryMut::{Occupied, Vacant};

        let xs = [(1_i32, 10_i32), (2, 20), (3, 30), (4, 40), (5, 50), (6, 60)];

        let mut map: HashMap<_, _> = xs.iter().copied().collect();

        let compute_hash = |map: &HashMap<i32, i32>, k: i32| -> u64 {
            super::make_hash::<i32, _>(map.hasher(), &k)
        };

        // Existing key (insert)
        match map.raw_entry_mut().from_key(&1) {
            Vacant(_) => unreachable!(),
            Occupied(mut view) => {
                assert_eq!(view.get(), &10);
                assert_eq!(view.insert(100), 10);
            }
        }
        let hash1 = compute_hash(&map, 1);
        assert_eq!(map.raw_entry().from_key(&1).unwrap(), (&1, &100));
        assert_eq!(
            map.raw_entry().from_hash(hash1, |k| *k == 1).unwrap(),
            (&1, &100)
        );
        assert_eq!(
            map.raw_entry().from_key_hashed_nocheck(hash1, &1).unwrap(),
            (&1, &100)
        );
        assert_eq!(map.len(), 6);

        // Existing key (update)
        match map.raw_entry_mut().from_key(&2) {
            Vacant(_) => unreachable!(),
            Occupied(mut view) => {
                let v = view.get_mut();
                let new_v = (*v) * 10;
                *v = new_v;
            }
        }
        let hash2 = compute_hash(&map, 2);
        assert_eq!(map.raw_entry().from_key(&2).unwrap(), (&2, &200));
        assert_eq!(
            map.raw_entry().from_hash(hash2, |k| *k == 2).unwrap(),
            (&2, &200)
        );
        assert_eq!(
            map.raw_entry().from_key_hashed_nocheck(hash2, &2).unwrap(),
            (&2, &200)
        );
        assert_eq!(map.len(), 6);

        // Existing key (take)
        let hash3 = compute_hash(&map, 3);
        match map.raw_entry_mut().from_key_hashed_nocheck(hash3, &3) {
            Vacant(_) => unreachable!(),
            Occupied(view) => {
                assert_eq!(view.remove_entry(), (3, 30));
            }
        }
        assert_eq!(map.raw_entry().from_key(&3), None);
        assert_eq!(map.raw_entry().from_hash(hash3, |k| *k == 3), None);
        assert_eq!(map.raw_entry().from_key_hashed_nocheck(hash3, &3), None);
        assert_eq!(map.len(), 5);

        // Nonexistent key (insert)
        match map.raw_entry_mut().from_key(&10) {
            Occupied(_) => unreachable!(),
            Vacant(view) => {
                assert_eq!(view.insert(10, 1000), (&mut 10, &mut 1000));
            }
        }
        assert_eq!(map.raw_entry().from_key(&10).unwrap(), (&10, &1000));
        assert_eq!(map.len(), 6);

        // Ensure all lookup methods produce equivalent results.
        for k in 0..12 {
            let hash = compute_hash(&map, k);
            let v = map.get(&k).copied();
            let kv = v.as_ref().map(|v| (&k, v));

            assert_eq!(map.raw_entry().from_key(&k), kv);
            assert_eq!(map.raw_entry().from_hash(hash, |q| *q == k), kv);
            assert_eq!(map.raw_entry().from_key_hashed_nocheck(hash, &k), kv);

            match map.raw_entry_mut().from_key(&k) {
                Occupied(o) => assert_eq!(Some(o.get_key_value()), kv),
                Vacant(_) => assert_eq!(v, None),
            }
            match map.raw_entry_mut().from_key_hashed_nocheck(hash, &k) {
                Occupied(o) => assert_eq!(Some(o.get_key_value()), kv),
                Vacant(_) => assert_eq!(v, None),
            }
            match map.raw_entry_mut().from_hash(hash, |q| *q == k) {
                Occupied(o) => assert_eq!(Some(o.get_key_value()), kv),
                Vacant(_) => assert_eq!(v, None),
            }
        }
    }

    #[test]
    fn test_key_without_hash_impl() {
        #[derive(Debug)]
        struct IntWrapper(u64);

        let mut m: HashMap<IntWrapper, (), ()> = HashMap::default();
        {
            assert!(m.raw_entry().from_hash(0, |k| k.0 == 0).is_none());
        }
        {
            let vacant_entry = match m.raw_entry_mut().from_hash(0, |k| k.0 == 0) {
                RawEntryMut::Occupied(..) => panic!("Found entry for key 0"),
                RawEntryMut::Vacant(e) => e,
            };
            vacant_entry.insert_with_hasher(0, IntWrapper(0), (), |k| k.0);
        }
        {
            assert!(m.raw_entry().from_hash(0, |k| k.0 == 0).is_some());
            assert!(m.raw_entry().from_hash(1, |k| k.0 == 1).is_none());
            assert!(m.raw_entry().from_hash(2, |k| k.0 == 2).is_none());
        }
        {
            let vacant_entry = match m.raw_entry_mut().from_hash(1, |k| k.0 == 1) {
                RawEntryMut::Occupied(..) => panic!("Found entry for key 1"),
                RawEntryMut::Vacant(e) => e,
            };
            vacant_entry.insert_with_hasher(1, IntWrapper(1), (), |k| k.0);
        }
        {
            assert!(m.raw_entry().from_hash(0, |k| k.0 == 0).is_some());
            assert!(m.raw_entry().from_hash(1, |k| k.0 == 1).is_some());
            assert!(m.raw_entry().from_hash(2, |k| k.0 == 2).is_none());
        }
        {
            let occupied_entry = match m.raw_entry_mut().from_hash(0, |k| k.0 == 0) {
                RawEntryMut::Occupied(e) => e,
                RawEntryMut::Vacant(..) => panic!("Couldn't find entry for key 0"),
            };
            occupied_entry.remove();
        }
        assert!(m.raw_entry().from_hash(0, |k| k.0 == 0).is_none());
        assert!(m.raw_entry().from_hash(1, |k| k.0 == 1).is_some());
        assert!(m.raw_entry().from_hash(2, |k| k.0 == 2).is_none());
    }
}
