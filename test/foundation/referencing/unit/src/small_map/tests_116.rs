#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_small() {
        let map: SmallMap<u32, u32> = SmallMap::new();
        assert!(matches!(map, SmallMap::Small(_)));
    }

    #[test]
    fn test_insert_and_get() {
        let mut map: SmallMap<String, u32> = SmallMap::new();
        map.insert("key".to_string(), 42);
        assert_eq!(map.get("key"), Some(&42));
        assert_eq!(map.get("missing"), None);
    }

    #[test]
    fn test_duplicate_key_overwrites() {
        let mut map: SmallMap<String, u32> = SmallMap::new();
        map.insert("key".to_string(), 1);
        map.insert("key".to_string(), 2);
        assert_eq!(map.get("key"), Some(&2));
        // Verify no duplicate was added: inserting the same key again should still return 2
        map.insert("key".to_string(), 2);
        assert_eq!(map.get("key"), Some(&2));
    }

    #[test]
    fn test_multiple_inserts_stay_small() {
        let mut map: SmallMap<u32, u32, 4> = SmallMap::new();
        for i in 0..4 {
            map.insert(i, i * 10);
        }
        assert!(matches!(map, SmallMap::Small(_)));
        for i in 0..4 {
            assert_eq!(map.get(&i), Some(&(i * 10)));
        }
    }

    #[test]
    fn test_promotion_at_n_plus_1() {
        let mut map: SmallMap<u32, u32, 4> = SmallMap::new();
        for i in 0..5 {
            map.insert(i, i * 10);
        }
        assert!(matches!(map, SmallMap::Large(_)));
        for i in 0..5 {
            assert_eq!(map.get(&i), Some(&(i * 10)));
        }
    }

    #[test]
    fn test_get_or_insert_default_miss() {
        let mut map: SmallMap<String, Vec<u32>> = SmallMap::new();
        map.get_or_insert_default("key".to_string()).push(1);
        assert_eq!(map.get("key"), Some(&vec![1u32]));
    }

    #[test]
    fn test_get_or_insert_default_hit() {
        let mut map: SmallMap<String, u32> = SmallMap::new();
        map.insert("key".to_string(), 42);
        let v = map.get_or_insert_default("key".to_string());
        assert_eq!(*v, 42);
        // Verify key was not duplicated: original value still accessible
        assert_eq!(map.get("key"), Some(&42));
    }

    #[test]
    fn test_get_or_insert_default_promotes() {
        let mut map: SmallMap<u32, u32, 2> = SmallMap::new();
        map.insert(1, 10);
        map.insert(2, 20);
        // map is full; inserting new key via get_or_insert_default should promote
        *map.get_or_insert_default(3) = 30;
        assert!(matches!(map, SmallMap::Large(_)));
        assert_eq!(map.get(&3), Some(&30));
        assert_eq!(map.get(&1), Some(&10));
        assert_eq!(map.get(&2), Some(&20));
    }

    #[test]
    fn test_nested_map() {
        let mut outer: SmallMap<u32, SmallMap<String, u32>> = SmallMap::new();
        outer.get_or_insert_default(1).insert("a".to_string(), 10);
        outer.get_or_insert_default(1).insert("b".to_string(), 20);
        outer.get_or_insert_default(2).insert("c".to_string(), 30);
        assert_eq!(outer.get(&1).unwrap().get("a"), Some(&10));
        assert_eq!(outer.get(&1).unwrap().get("b"), Some(&20));
        assert_eq!(outer.get(&2).unwrap().get("c"), Some(&30));
    }

    #[test]
    fn test_clone() {
        let mut map: SmallMap<u32, u32, 4> = SmallMap::new();
        map.insert(1, 10);
        map.insert(2, 20);
        let cloned = map.clone();
        assert_eq!(cloned.get(&1), Some(&10));
        assert_eq!(cloned.get(&2), Some(&20));
    }

    #[test]
    fn test_clone_large() {
        let mut map: SmallMap<u32, u32, 4> = SmallMap::new();
        for i in 0..5 {
            map.insert(i, i * 10);
        }
        let cloned = map.clone();
        for i in 0..5 {
            assert_eq!(cloned.get(&i), Some(&(i * 10)));
        }
    }
}
