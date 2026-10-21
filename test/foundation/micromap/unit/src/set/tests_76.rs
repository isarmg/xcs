#[cfg(test)]
mod tests {
    use super::Set;

    /// This is a deliberately lengthy test function.
    /// As a test under the mod root path, it is necessary to consider the impact
    /// of previous operations on subsequent operations. Some bugs are triggered
    /// only when they accumulate. This function is used to test whether there
    /// are such bugs. And no blank line separation is the rule for this project.
    #[test]
    fn various() {
        let mut set: Set<i32, 10> = Set::new();
        let set_default: Set<i32, 10> = Set::default();
        assert!(set == set_default);
        assert_eq!(set.capacity(), 10);
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
        set.insert(0);
        assert_eq!(set.len(), 1);
        set.insert(0);
        assert_eq!(set.len(), 1);
        set.insert(1);
        assert_eq!(set.len(), 2);
        let mut drain = set.drain();
        assert_eq!(drain.len(), 2);
        assert!(drain.next().is_some());
        assert_eq!(drain.len(), 1);
        assert_eq!(drain.len(), drain.size_hint().0);
        assert_eq!(drain.len(), drain.size_hint().1.unwrap());
        drop(drain);
        set.clear();
        for n in 0..10 {
            set.insert(n);
        }
        assert_eq!(set.len(), 10);
        assert!(set.contains(&0));
        assert_eq!(set.take(&0), Some(0));
        assert!(!set.contains(&10));
        assert_eq!(set.remove(&0), false);
        set.insert(0);
        assert_eq!(set.remove(&0), true);
        set.insert(10);
        assert_eq!(set.get(&10), Some(&10));
        assert_eq!(set.get(&11), None);
        set.retain(|&k| k % 2 == 0);
        let mut it = set.iter();
        assert!(it.next().is_some());
        assert_eq!(it.len(), 4);
        assert_eq!(it.len(), it.size_hint().0);
        assert_eq!(it.len(), it.size_hint().1.unwrap());
        let mut it_into = set.into_iter();
        assert!(it_into.next().is_some());
        assert!(it_into.next().is_some());
        assert!(it_into.next().is_some());
        assert_eq!(it_into.len(), 2);
        assert_eq!(it_into.len(), it_into.size_hint().0);
        assert_eq!(it_into.len(), it_into.size_hint().1.unwrap());
    }

    #[test]
    fn test_set_from() {
        let set_a = Set::from(['a', 'b', 'c', 'd']);
        let set_b: Set<_, 6> = Set::from_iter(['a', 'a', 'd', 'b', 'a', 'd', 'c', 'd', 'c']);
        let set_c = set_a.clone();
        assert_eq!(set_a, set_b);
        assert_eq!(set_a, set_c);
    }
}
