#[cfg(test)]
mod tests {
    use super::*;
    use std::string::String;

    #[test]
    fn insert_order() {
        let insert = [0, 4, 2, 12, 8, 7, 11, 5, 3, 17, 19, 22, 23];
        let mut map = IndexMap::new();

        for &elt in &insert {
            map.insert(elt, ());
        }

        assert_eq!(map.par_keys().count(), map.len());
        assert_eq!(map.par_keys().count(), insert.len());
        insert.par_iter().zip(map.par_keys()).for_each(|(a, b)| {
            assert_eq!(a, b);
        });
        (0..insert.len())
            .into_par_iter()
            .zip(map.par_keys())
            .for_each(|(i, k)| {
                assert_eq!(map.get_index(i).unwrap().0, k);
            });
    }

    #[test]
    fn partial_eq_and_eq() {
        let mut map_a = IndexMap::new();
        map_a.insert(1, "1");
        map_a.insert(2, "2");
        let mut map_b = map_a.clone();
        assert!(map_a.par_eq(&map_b));
        map_b.swap_remove(&1);
        assert!(!map_a.par_eq(&map_b));
        map_b.insert(3, "3");
        assert!(!map_a.par_eq(&map_b));

        let map_c: IndexMap<_, String> =
            map_b.into_par_iter().map(|(k, v)| (k, v.into())).collect();
        assert!(!map_a.par_eq(&map_c));
        assert!(!map_c.par_eq(&map_a));
    }

    #[test]
    fn extend() {
        let mut map = IndexMap::new();
        map.par_extend(vec![(&1, &2), (&3, &4)]);
        map.par_extend(vec![(5, 6)]);
        assert_eq!(
            map.into_par_iter().collect::<Vec<_>>(),
            vec![(1, 2), (3, 4), (5, 6)]
        );
    }

    #[test]
    fn keys() {
        let vec = vec![(1, 'a'), (2, 'b'), (3, 'c')];
        let map: IndexMap<_, _> = vec.into_par_iter().collect();
        let keys: Vec<_> = map.par_keys().copied().collect();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&1));
        assert!(keys.contains(&2));
        assert!(keys.contains(&3));
    }

    #[test]
    fn values() {
        let vec = vec![(1, 'a'), (2, 'b'), (3, 'c')];
        let map: IndexMap<_, _> = vec.into_par_iter().collect();
        let values: Vec<_> = map.par_values().copied().collect();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&'a'));
        assert!(values.contains(&'b'));
        assert!(values.contains(&'c'));
    }

    #[test]
    fn values_mut() {
        let vec = vec![(1, 1), (2, 2), (3, 3)];
        let mut map: IndexMap<_, _> = vec.into_par_iter().collect();
        map.par_values_mut().for_each(|value| *value *= 2);
        let values: Vec<_> = map.par_values().copied().collect();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&2));
        assert!(values.contains(&4));
        assert!(values.contains(&6));
    }
}
