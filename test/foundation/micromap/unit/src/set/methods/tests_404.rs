#[cfg(test)]
mod tests {
    use super::Set;

    #[test]
    fn test_capacity() {
        let set: Set<i32, 10> = Set::new();
        assert_eq!(set.capacity(), 10);
    }

    #[test]
    fn test_is_empty() {
        let mut set: Set<i32, 5> = Set::new();
        assert!(set.is_empty());
        set.insert(1);
        assert!(!set.is_empty());
    }

    #[test]
    fn test_len() {
        let mut set: Set<i32, 5> = Set::new();
        assert_eq!(set.len(), 0);
        set.insert(1);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut set: Set<i32, 5> = Set::new();
        set.insert(1);
        set.clear();
        assert!(set.is_empty());
    }

    #[test]
    fn test_retain() {
        let mut set = Set::from([1, 2, 3, 4, 5]);
        set.retain(|&x| x % 2 == 0);
        assert_eq!(set, Set::from([2, 4]));
    }

    #[test]
    fn test_contains() {
        let set = Set::from([1, 2, 3]);
        assert!(set.contains(&1));
        assert!(!set.contains(&4));
    }

    #[test]
    fn test_remove() {
        let mut set: Set<i32, 5> = Set::new();
        set.insert(2);
        assert!(set.remove(&2));
        assert!(!set.remove(&2));
    }

    #[test]
    fn test_insert() {
        let mut set: Set<i32, 3> = Set::new();
        assert!(set.insert(2));
        assert!(!set.insert(2));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_checked_insert() {
        let mut set: Set<i32, 2> = Set::new();
        assert_eq!(set.checked_insert(1), None);
        assert_eq!(set.checked_insert(2), None);
        assert_eq!(set.checked_insert(3), None);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_insert_unchecked() {
        let mut set: Set<i32, 3> = Set::new();
        unsafe {
            assert!(set.insert_unchecked(1));
            assert!(set.insert_unchecked(2));
            assert!(!set.insert_unchecked(2));
        }
    }

    #[test]
    fn test_get() {
        let set = Set::from([1, 2, 3]);
        assert_eq!(set.get(&2), Some(&2));
        assert_eq!(set.get(&4), None);
    }

    #[test]
    fn test_take() {
        let mut set = Set::from([1, 2, 3]);
        assert_eq!(set.take(&2), Some(2));
        assert_eq!(set.take(&2), None);
    }

    #[test]
    fn test_is_disjoint() {
        let a = Set::from([1, 2, 3]);
        let mut b: Set<u32, 5> = Set::new();
        assert!(a.is_disjoint(&b));
        assert!(b.is_disjoint(&a));
        b.insert(4);
        assert!(a.is_disjoint(&b));
        assert!(b.is_disjoint(&a));
        b.insert(1);
        assert!(!a.is_disjoint(&b));
        assert!(!b.is_disjoint(&a));
    }

    #[test]
    fn test_is_subset() {
        let sup = Set::from([1, 2, 3]);
        let mut set: Set<u32, 5> = Set::new();
        assert!(set.is_subset(&sup));
        set.insert(2);
        assert!(set.is_subset(&sup));
        set.insert(4);
        assert!(!set.is_subset(&sup));
    }

    #[test]
    fn test_is_superset() {
        let sub = Set::from([1, 2]);
        let mut set: Set<u32, 5> = Set::new();
        assert!(!set.is_superset(&sub));
        set.insert(1);
        set.insert(2);
        assert!(set.is_superset(&sub));
    }

    #[test]
    fn test_replace() {
        let mut set: Set<Vec<i32>, 5> = Set::new();
        set.insert(Vec::new());
        assert_eq!(set.get(&[][..]).unwrap().capacity(), 0);
        set.replace(Vec::with_capacity(10));
        assert_eq!(set.get(&[][..]).unwrap().capacity(), 10);
    }
}
