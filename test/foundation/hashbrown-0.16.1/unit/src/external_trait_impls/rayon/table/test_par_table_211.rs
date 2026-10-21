#[cfg(test)]
mod test_par_table {
    use alloc::vec::Vec;
    use core::sync::atomic::{AtomicUsize, Ordering};

    use rayon::prelude::*;

    use crate::{hash_map::make_hash, hash_table::HashTable, DefaultHashBuilder};

    #[test]
    fn test_iterate() {
        let hasher = DefaultHashBuilder::default();
        let mut a = HashTable::new();
        for i in 0..32 {
            a.insert_unique(make_hash(&hasher, &i), i, |x| make_hash(&hasher, x));
        }
        let observed = AtomicUsize::new(0);
        a.par_iter().for_each(|k| {
            observed.fetch_or(1 << *k, Ordering::Relaxed);
        });
        assert_eq!(observed.into_inner(), 0xFFFF_FFFF);
    }

    #[test]
    fn test_move_iter() {
        let hasher = DefaultHashBuilder::default();
        let hs = {
            let mut hs = HashTable::new();

            hs.insert_unique(make_hash(&hasher, &'a'), 'a', |x| make_hash(&hasher, x));
            hs.insert_unique(make_hash(&hasher, &'b'), 'b', |x| make_hash(&hasher, x));

            hs
        };

        let v = hs.into_par_iter().collect::<Vec<char>>();
        assert!(v == ['a', 'b'] || v == ['b', 'a']);
    }
}
