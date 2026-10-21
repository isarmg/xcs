#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use quickcheck::quickcheck;

    use super::*;

    #[derive(Clone)]
    struct CountDrop<'a>(&'a std::cell::RefCell<usize>);

    impl<'a> Drop for CountDrop<'a> {
        fn drop(&mut self) {
            *self.0.borrow_mut() += 1;
        }
    }

    #[test]
    fn check_drops() {
        let drops = std::cell::RefCell::new(0usize);

        {
            let mut clone = {
                // Insert 1000 items.
                let mut sm = SlotMap::new();
                let mut sm_keys = Vec::new();
                for _ in 0..1000 {
                    sm_keys.push(sm.insert(CountDrop(&drops)));
                }

                // Remove even keys.
                for i in (0..1000).filter(|i| i % 2 == 0) {
                    sm.remove(sm_keys[i]);
                }

                // Should only have dropped 500 so far.
                assert_eq!(*drops.borrow(), 500);

                // Let's clone ourselves and then die.
                sm.clone()
            };

            // Now all original items should have been dropped exactly once.
            assert_eq!(*drops.borrow(), 1000);

            // Reuse some empty slots.
            for _ in 0..250 {
                clone.insert(CountDrop(&drops));
            }
        }

        // 1000 + 750 drops in total should have happened.
        assert_eq!(*drops.borrow(), 1750);
    }

    #[test]
    fn disjoint() {
        // Intended to be run with miri to find any potential UB.
        let mut sm = SlotMap::new();

        // Some churn.
        for i in 0..20usize {
            sm.insert(i);
        }
        sm.retain(|_, i| *i % 2 == 0);

        let keys: Vec<_> = sm.keys().collect();
        for i in 0..keys.len() {
            for j in 0..keys.len() {
                if let Some([r0, r1]) = sm.get_disjoint_mut([keys[i], keys[j]]) {
                    *r0 ^= *r1;
                    *r1 = r1.wrapping_add(*r0);
                } else {
                    assert!(i == j);
                }
            }
        }

        for i in 0..keys.len() {
            for j in 0..keys.len() {
                for k in 0..keys.len() {
                    if let Some([r0, r1, r2]) = sm.get_disjoint_mut([keys[i], keys[j], keys[k]]) {
                        *r0 ^= *r1;
                        *r0 = r0.wrapping_add(*r2);
                        *r1 ^= *r0;
                        *r1 = r1.wrapping_add(*r2);
                        *r2 ^= *r0;
                        *r2 = r2.wrapping_add(*r1);
                    } else {
                        assert!(i == j || j == k || i == k);
                    }
                }
            }
        }
    }

    quickcheck! {
        fn qc_slotmap_equiv_hashmap(operations: Vec<(u8, u32)>) -> bool {
            let mut hm = HashMap::new();
            let mut hm_keys = Vec::new();
            let mut unique_key = 0u32;
            let mut sm = SlotMap::new();
            let mut sm_keys = Vec::new();

            #[cfg(not(feature = "serialization"))]
            let num_ops = 3;
            #[cfg(feature = "serialization")]
            let num_ops = 4;

            for (op, val) in operations {
                match op % num_ops {
                    // Insert.
                    0 => {
                        hm.insert(unique_key, val);
                        hm_keys.push(unique_key);
                        unique_key += 1;

                        sm_keys.push(sm.insert(val));
                    }

                    // Delete.
                    1 => {
                        // 10% of the time test clear.
                        if val % 10 == 0 {
                            let hmvals: HashSet<_> = hm.drain().map(|(_, v)| v).collect();
                            let smvals: HashSet<_> = sm.drain().map(|(_, v)| v).collect();
                            if hmvals != smvals {
                                return false;
                            }
                        }
                        if hm_keys.is_empty() { continue; }

                        let idx = val as usize % hm_keys.len();
                        if hm.remove(&hm_keys[idx]) != sm.remove(sm_keys[idx]) {
                            return false;
                        }
                    }

                    // Access.
                    2 => {
                        if hm_keys.is_empty() { continue; }
                        let idx = val as usize % hm_keys.len();
                        let (hm_key, sm_key) = (&hm_keys[idx], sm_keys[idx]);

                        if hm.contains_key(hm_key) != sm.contains_key(sm_key) ||
                           hm.get(hm_key) != sm.get(sm_key) {
                            return false;
                        }
                    }

                    // Serde round-trip.
                    #[cfg(feature = "serialization")]
                    3 => {
                        let ser = json_codec::to_string(&sm).unwrap();
                        sm = json_codec::from_str(&ser).unwrap();
                    }

                    _ => unreachable!(),
                }
            }

            let mut smv: Vec<_> = sm.values().collect();
            let mut hmv: Vec<_> = hm.values().collect();
            smv.sort();
            hmv.sort();
            smv == hmv
        }
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn slotmap_serialization() {
        let mut sm = SlotMap::new();
        // Self-referential structure.
        let first = sm.insert_with_key(|k| (k, 23i32));
        let second = sm.insert((first, 42));

        // Make some empty slots.
        let empties = vec![sm.insert((first, 0)), sm.insert((first, 0))];
        empties.iter().for_each(|k| {
            sm.remove(*k);
        });

        let third = sm.insert((second, 0));
        sm[first].0 = third;

        let ser = json_codec::to_string(&sm).unwrap();
        let de: SlotMap<DefaultKey, (DefaultKey, i32)> = json_codec::from_str(&ser).unwrap();
        assert_eq!(de.len(), sm.len());

        let mut smkv: Vec<_> = sm.iter().collect();
        let mut dekv: Vec<_> = de.iter().collect();
        smkv.sort();
        dekv.sort();
        assert_eq!(smkv, dekv);
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn slotmap_serialization_freelist() {
        let mut sm = SlotMap::new();
        let k = sm.insert(5i32);
        sm.remove(k);

        let ser = json_codec::to_string(&sm).unwrap();
        let mut de: SlotMap<DefaultKey, i32> = json_codec::from_str(&ser).unwrap();

        de.insert(0);
        de.insert(1);
        de.insert(2);
        assert_eq!(de.len(), 3);
    }
}
