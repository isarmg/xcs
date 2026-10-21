#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_order() {
        let insert = [0, 4, 2, 12, 8, 7, 11, 5, 3, 17, 19, 22, 23];
        let mut set = IndexSet::new();

        for &elt in &insert {
            set.insert(elt);
        }

        assert_eq!(set.par_iter().count(), set.len());
        assert_eq!(set.par_iter().count(), insert.len());
        insert.par_iter().zip(&set).for_each(|(a, b)| {
            assert_eq!(a, b);
        });
        (0..insert.len())
            .into_par_iter()
            .zip(&set)
            .for_each(|(i, v)| {
                assert_eq!(set.get_index(i).unwrap(), v);
            });
    }

    #[test]
    fn partial_eq_and_eq() {
        let mut set_a = IndexSet::new();
        set_a.insert(1);
        set_a.insert(2);
        let mut set_b = set_a.clone();
        assert!(set_a.par_eq(&set_b));
        set_b.swap_remove(&1);
        assert!(!set_a.par_eq(&set_b));
        set_b.insert(3);
        assert!(!set_a.par_eq(&set_b));

        let set_c: IndexSet<_> = set_b.into_par_iter().collect();
        assert!(!set_a.par_eq(&set_c));
        assert!(!set_c.par_eq(&set_a));
    }

    #[test]
    fn extend() {
        let mut set = IndexSet::new();
        set.par_extend(vec![&1, &2, &3, &4]);
        set.par_extend(vec![5, 6]);
        assert_eq!(
            set.into_par_iter().collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn comparisons() {
        let set_a: IndexSet<_> = (0..3).collect();
        let set_b: IndexSet<_> = (3..6).collect();
        let set_c: IndexSet<_> = (0..6).collect();
        let set_d: IndexSet<_> = (3..9).collect();

        assert!(!set_a.par_is_disjoint(&set_a));
        assert!(set_a.par_is_subset(&set_a));
        assert!(set_a.par_is_superset(&set_a));

        assert!(set_a.par_is_disjoint(&set_b));
        assert!(set_b.par_is_disjoint(&set_a));
        assert!(!set_a.par_is_subset(&set_b));
        assert!(!set_b.par_is_subset(&set_a));
        assert!(!set_a.par_is_superset(&set_b));
        assert!(!set_b.par_is_superset(&set_a));

        assert!(!set_a.par_is_disjoint(&set_c));
        assert!(!set_c.par_is_disjoint(&set_a));
        assert!(set_a.par_is_subset(&set_c));
        assert!(!set_c.par_is_subset(&set_a));
        assert!(!set_a.par_is_superset(&set_c));
        assert!(set_c.par_is_superset(&set_a));

        assert!(!set_c.par_is_disjoint(&set_d));
        assert!(!set_d.par_is_disjoint(&set_c));
        assert!(!set_c.par_is_subset(&set_d));
        assert!(!set_d.par_is_subset(&set_c));
        assert!(!set_c.par_is_superset(&set_d));
        assert!(!set_d.par_is_superset(&set_c));
    }

    #[test]
    fn iter_comparisons() {
        use std::iter::empty;

        fn check<'a, I1, I2>(iter1: I1, iter2: I2)
        where
            I1: ParallelIterator<Item = &'a i32>,
            I2: Iterator<Item = i32>,
        {
            let v1: Vec<_> = iter1.copied().collect();
            let v2: Vec<_> = iter2.collect();
            assert_eq!(v1, v2);
        }

        let set_a: IndexSet<_> = (0..3).collect();
        let set_b: IndexSet<_> = (3..6).collect();
        let set_c: IndexSet<_> = (0..6).collect();
        let set_d: IndexSet<_> = (3..9).rev().collect();

        check(set_a.par_difference(&set_a), empty());
        check(set_a.par_symmetric_difference(&set_a), empty());
        check(set_a.par_intersection(&set_a), 0..3);
        check(set_a.par_union(&set_a), 0..3);

        check(set_a.par_difference(&set_b), 0..3);
        check(set_b.par_difference(&set_a), 3..6);
        check(set_a.par_symmetric_difference(&set_b), 0..6);
        check(set_b.par_symmetric_difference(&set_a), (3..6).chain(0..3));
        check(set_a.par_intersection(&set_b), empty());
        check(set_b.par_intersection(&set_a), empty());
        check(set_a.par_union(&set_b), 0..6);
        check(set_b.par_union(&set_a), (3..6).chain(0..3));

        check(set_a.par_difference(&set_c), empty());
        check(set_c.par_difference(&set_a), 3..6);
        check(set_a.par_symmetric_difference(&set_c), 3..6);
        check(set_c.par_symmetric_difference(&set_a), 3..6);
        check(set_a.par_intersection(&set_c), 0..3);
        check(set_c.par_intersection(&set_a), 0..3);
        check(set_a.par_union(&set_c), 0..6);
        check(set_c.par_union(&set_a), 0..6);

        check(set_c.par_difference(&set_d), 0..3);
        check(set_d.par_difference(&set_c), (6..9).rev());
        check(
            set_c.par_symmetric_difference(&set_d),
            (0..3).chain((6..9).rev()),
        );
        check(
            set_d.par_symmetric_difference(&set_c),
            (6..9).rev().chain(0..3),
        );
        check(set_c.par_intersection(&set_d), 3..6);
        check(set_d.par_intersection(&set_c), (3..6).rev());
        check(set_c.par_union(&set_d), (0..6).chain((6..9).rev()));
        check(set_d.par_union(&set_c), (3..9).rev().chain(0..3));
    }
}
