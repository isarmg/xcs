#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use quickcheck::quickcheck;

    use crate::*;

    #[test]
    fn disjoint() {
        // Intended to be run with miri to find any potential UB.
        let mut sm = SlotMap::new();
        let mut sec = SecondaryMap::new();

        // Some churn.
        for i in 0..20usize {
            sm.insert(i);
        }
        sm.retain(|_, i| *i % 2 == 0);

        for (i, k) in sm.keys().enumerate() {
            sec.insert(k, i);
        }

        let keys: Vec<_> = sm.keys().collect();
        for i in 0..keys.len() {
            for j in 0..keys.len() {
                if let Some([r0, r1]) = sec.get_disjoint_mut([keys[i], keys[j]]) {
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
                    if let Some([r0, r1, r2]) = sec.get_disjoint_mut([keys[i], keys[j], keys[k]]) {
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
        fn qc_secmap_equiv_hashmap(operations: Vec<(u8, u32)>) -> bool {
            let mut hm = HashMap::new();
            let mut hm_keys = Vec::new();
            let mut unique_key = 0u32;
            let mut sm = SlotMap::new();
            let mut sec = SecondaryMap::new();
            let mut sm_keys = Vec::new();

            #[cfg(not(feature = "serialization"))]
            let num_ops = 4;
            #[cfg(feature = "serialization")]
            let num_ops = 5;

            for (op, val) in operations {
                match op % num_ops {
                    // Insert.
                    0 => {
                        hm.insert(unique_key, val);
                        hm_keys.push(unique_key);
                        unique_key += 1;

                        let k = sm.insert(val);
                        sec.insert(k, val);
                        sm_keys.push(k);
                    }

                    // Delete.
                    1 => {
                        if hm_keys.is_empty() { continue; }

                        let idx = val as usize % hm_keys.len();
                        sm.remove(sm_keys[idx]);
                        if hm.remove(&hm_keys[idx]) != sec.remove(sm_keys[idx]) {
                            return false;
                        }
                    }

                    // Access.
                    2 => {
                        if hm_keys.is_empty() { continue; }
                        let idx = val as usize % hm_keys.len();
                        let (hm_key, sm_key) = (&hm_keys[idx], sm_keys[idx]);

                        if hm.contains_key(hm_key) != sec.contains_key(sm_key) ||
                           hm.get(hm_key) != sec.get(sm_key) {
                            return false;
                        }
                    }

                    // Clone.
                    3 => {
                        sec = sec.clone();
                    }

                    // Serde round-trip.
                    #[cfg(feature = "serialization")]
                    4 => {
                        let ser = json_codec::to_string(&sec).unwrap();
                        sec = json_codec::from_str(&ser).unwrap();
                    }

                    _ => unreachable!(),
                }
            }

            let mut secv: Vec<_> = sec.values().collect();
            let mut hmv: Vec<_> = hm.values().collect();
            secv.sort();
            hmv.sort();
            secv == hmv
        }
    }
}
