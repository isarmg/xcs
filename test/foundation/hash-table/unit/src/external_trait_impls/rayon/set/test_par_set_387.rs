#[cfg(test)]
mod test_par_set {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use stdalloc::vec::Vec;

    use rayon::prelude::*;

    use crate::HashSet;

    #[test]
    fn test_disjoint() {
        let mut xs = HashSet::new();
        let mut ys = HashSet::new();
        assert!(xs.par_is_disjoint(&ys));
        assert!(ys.par_is_disjoint(&xs));
        assert!(xs.insert(5));
        assert!(ys.insert(11));
        assert!(xs.par_is_disjoint(&ys));
        assert!(ys.par_is_disjoint(&xs));
        assert!(xs.insert(7));
        assert!(xs.insert(19));
        assert!(xs.insert(4));
        assert!(ys.insert(2));
        assert!(ys.insert(-11));
        assert!(xs.par_is_disjoint(&ys));
        assert!(ys.par_is_disjoint(&xs));
        assert!(ys.insert(7));
        assert!(!xs.par_is_disjoint(&ys));
        assert!(!ys.par_is_disjoint(&xs));
    }

    #[test]
    fn test_subset_and_superset() {
        let mut a = HashSet::new();
        assert!(a.insert(0));
        assert!(a.insert(5));
        assert!(a.insert(11));
        assert!(a.insert(7));

        let mut b = HashSet::new();
        assert!(b.insert(0));
        assert!(b.insert(7));
        assert!(b.insert(19));
        assert!(b.insert(250));
        assert!(b.insert(11));
        assert!(b.insert(200));

        assert!(!a.par_is_subset(&b));
        assert!(!a.par_is_superset(&b));
        assert!(!b.par_is_subset(&a));
        assert!(!b.par_is_superset(&a));

        assert!(b.insert(5));

        assert!(a.par_is_subset(&b));
        assert!(!a.par_is_superset(&b));
        assert!(!b.par_is_subset(&a));
        assert!(b.par_is_superset(&a));
    }

    #[test]
    fn test_iterate() {
        let mut a = HashSet::new();
        for i in 0..32 {
            assert!(a.insert(i));
        }
        let observed = AtomicUsize::new(0);
        a.par_iter().for_each(|k| {
            observed.fetch_or(1 << *k, Ordering::Relaxed);
        });
        assert_eq!(observed.into_inner(), 0xFFFF_FFFF);
    }

    #[test]
    fn test_intersection() {
        let mut a = HashSet::new();
        let mut b = HashSet::new();

        assert!(a.insert(11));
        assert!(a.insert(1));
        assert!(a.insert(3));
        assert!(a.insert(77));
        assert!(a.insert(103));
        assert!(a.insert(5));
        assert!(a.insert(-5));

        assert!(b.insert(2));
        assert!(b.insert(11));
        assert!(b.insert(77));
        assert!(b.insert(-9));
        assert!(b.insert(-42));
        assert!(b.insert(5));
        assert!(b.insert(3));

        let expected = [3, 5, 11, 77];
        let i = a
            .par_intersection(&b)
            .map(|x| {
                assert!(expected.contains(x));
                1
            })
            .sum::<usize>();
        assert_eq!(i, expected.len());
    }

    #[test]
    fn test_difference() {
        let mut a = HashSet::new();
        let mut b = HashSet::new();

        assert!(a.insert(1));
        assert!(a.insert(3));
        assert!(a.insert(5));
        assert!(a.insert(9));
        assert!(a.insert(11));

        assert!(b.insert(3));
        assert!(b.insert(9));

        let expected = [1, 5, 11];
        let i = a
            .par_difference(&b)
            .map(|x| {
                assert!(expected.contains(x));
                1
            })
            .sum::<usize>();
        assert_eq!(i, expected.len());
    }

    #[test]
    fn test_symmetric_difference() {
        let mut a = HashSet::new();
        let mut b = HashSet::new();

        assert!(a.insert(1));
        assert!(a.insert(3));
        assert!(a.insert(5));
        assert!(a.insert(9));
        assert!(a.insert(11));

        assert!(b.insert(-2));
        assert!(b.insert(3));
        assert!(b.insert(9));
        assert!(b.insert(14));
        assert!(b.insert(22));

        let expected = [-2, 1, 5, 11, 14, 22];
        let i = a
            .par_symmetric_difference(&b)
            .map(|x| {
                assert!(expected.contains(x));
                1
            })
            .sum::<usize>();
        assert_eq!(i, expected.len());
    }

    #[test]
    fn test_union() {
        let mut a = HashSet::new();
        let mut b = HashSet::new();

        assert!(a.insert(1));
        assert!(a.insert(3));
        assert!(a.insert(5));
        assert!(a.insert(9));
        assert!(a.insert(11));
        assert!(a.insert(16));
        assert!(a.insert(19));
        assert!(a.insert(24));

        assert!(b.insert(-2));
        assert!(b.insert(1));
        assert!(b.insert(5));
        assert!(b.insert(9));
        assert!(b.insert(13));
        assert!(b.insert(19));

        let expected = [-2, 1, 3, 5, 9, 11, 13, 16, 19, 24];
        let i = a
            .par_union(&b)
            .map(|x| {
                assert!(expected.contains(x));
                1
            })
            .sum::<usize>();
        assert_eq!(i, expected.len());
    }

    #[test]
    fn test_from_iter() {
        let xs = [1, 2, 3, 4, 5, 6, 7, 8, 9];

        let set: HashSet<_> = xs.par_iter().cloned().collect();

        for x in &xs {
            assert!(set.contains(x));
        }
    }

    #[test]
    fn test_move_iter() {
        let hs = {
            let mut hs = HashSet::new();

            hs.insert('a');
            hs.insert('b');

            hs
        };

        let v = hs.into_par_iter().collect::<Vec<char>>();
        assert!(v == ['a', 'b'] || v == ['b', 'a']);
    }

    #[test]
    fn test_eq() {
        // These constants once happened to expose a bug in insert().
        // I'm keeping them around to prevent a regression.
        let mut s1 = HashSet::new();

        s1.insert(1);
        s1.insert(2);
        s1.insert(3);

        let mut s2 = HashSet::new();

        s2.insert(1);
        s2.insert(2);

        assert!(!s1.par_eq(&s2));

        s2.insert(3);

        assert!(s1.par_eq(&s2));
    }

    #[test]
    fn test_extend_ref() {
        let mut a = HashSet::new();
        a.insert(1);

        a.par_extend(&[2, 3, 4][..]);

        assert_eq!(a.len(), 4);
        assert!(a.contains(&1));
        assert!(a.contains(&2));
        assert!(a.contains(&3));
        assert!(a.contains(&4));

        let mut b = HashSet::new();
        b.insert(5);
        b.insert(6);

        a.par_extend(&b);

        assert_eq!(a.len(), 6);
        assert!(a.contains(&1));
        assert!(a.contains(&2));
        assert!(a.contains(&3));
        assert!(a.contains(&4));
        assert!(a.contains(&5));
        assert!(a.contains(&6));
    }
}
