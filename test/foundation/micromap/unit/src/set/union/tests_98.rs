#[cfg(test)]
mod tests {
    use super::Set;

    #[test]
    fn union_simple() {
        let set_a = Set::from([0, 1, 2, 3, 5, 7, 9]);
        let set_b = Set::from([2, 5, 6, 7, 8, 10]);
        let set_c = Set::from([0, 1, 2, 3, 5, 7, 9, 2, 5, 6, 7, 8]);
        let set_result: Set<_, 13> = set_a.union(&set_b).copied().collect();
        assert_eq!(set_result, Set::from([0, 1, 2, 3, 5, 6, 7, 8, 9, 10]));
        let set_result: Set<_, 19> = set_a.union(&set_c).copied().collect();
        assert_eq!(set_result, Set::from([0, 1, 2, 3, 5, 6, 7, 8, 9]));
        let set_result: Set<_, 18> = set_b.union(&set_c).copied().collect();
        assert_eq!(set_result, Set::from([0, 1, 2, 3, 5, 6, 7, 8, 9, 10]));
    }

    #[test]
    fn union_with_empty_set() {
        let set_a = Set::from([1, 2, 3]);
        let set_b: Set<i32, 3> = Set::new();
        let union: Set<_, 6> = set_a.union(&set_b).copied().collect();
        assert_eq!(union, set_a);
    }

    #[test]
    fn union_with_disjoint_sets() {
        let a = Set::from([1, 2, 3]);
        let b = Set::from([4, 5, 6]);
        let union: Set<_, 6> = a.union(&b).copied().collect();
        assert_eq!(union, Set::from([1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn union_with_subset() {
        let a = Set::from([1, 2, 3, 4]);
        let b = Set::from([2, 3]);
        let union: Set<_, 6> = a.union(&b).copied().collect();
        assert_eq!(union, Set::from([1, 2, 3, 4]));
    }

    #[test]
    fn union_with_superset() {
        let a = Set::from([2, 3]);
        let b = Set::from([1, 2, 3, 4]);
        let union: Set<_, 6> = a.union(&b).copied().collect();
        assert_eq!(union, Set::from([1, 2, 3, 4]));
    }

    #[test]
    fn union_size_hint() {
        let set_a = Set::from([1, 1, 2, 3]); // cap is 4, but len() is 3
        let set_b = Set::from([4, 5, 6, 6, 6, 7, 8, 9]); // cap is 8, but len() is 6
        let set_c = Set::from([]);
        let set_d = Set::from([3, 4]);
        assert_eq!(set_a.union(&set_b).size_hint(), (6, Some(9)));
        assert_eq!(set_a.union(&set_c).size_hint(), (3, Some(3)));
        assert_eq!(set_a.union(&set_d).size_hint(), (3, Some(5)));
        assert_eq!(set_b.union(&set_a).size_hint(), (6, Some(9)));
        assert_eq!(set_b.union(&set_d).size_hint(), (6, Some(8)));
        assert_eq!(set_c.union(&set_b).size_hint(), (6, Some(6)));
        assert_eq!(set_d.union(&set_a).size_hint(), (3, Some(5)));
        assert_eq!(set_d.union(&set_b).size_hint(), (6, Some(8)));
        assert_eq!(set_d.union(&set_c).size_hint(), (2, Some(2)));
    }

    #[test]
    fn union_default_and_clone() {
        let set_a = Set::from([1, 2, 3]);
        let set_b = Set::from([4, 5, 6]);
        let union = set_a.union(&set_b);
        let cloned_union = union.clone();
        assert_eq!(union.count(), cloned_union.count());
    }

    #[test]
    fn union_debug_fmt() {
        let set_a = Set::from([1, 2, 3]);
        let set_b = Set::from([4, 5, 6]);
        let union = set_a.union(&set_b);
        let debug_output = format!("{:?}", union);
        assert!(debug_output.contains("1"));
        assert!(debug_output.contains("6"));
    }

    #[test]
    fn union_count_and_fold() {
        let set_a = Set::from([1, 2, 3]);
        let set_b = Set::from([3, 4, 5]);
        let union = set_a.union(&set_b);
        assert_eq!(union.clone().count(), 5);
        let sum: i32 = union.fold(0, |acc, &x| acc + x);
        assert_eq!(sum, 15); // 1 + 2 + 3 + 4 + 5
    }

    #[test]
    fn union_combined_test() {
        let set_a = Set::from([1, 2, 3]);
        let set_b = Set::from([3, 4, 5]);
        let union = set_a.union(&set_b);
        // Test clone
        let cloned_union = union.clone();
        assert_eq!(union.clone().count(), cloned_union.count());
        // Test debug fmt
        let debug_output = format!("{:?}", union);
        assert!(debug_output.contains("1"));
        assert!(debug_output.contains("5"));
        // Test count and fold
        assert_eq!(union.clone().count(), 5);
        let sum: i32 = union.fold(0, |acc, &x| acc + x);
        assert_eq!(sum, 15); // 1 + 2 + 3 + 4 + 5
    }
}
