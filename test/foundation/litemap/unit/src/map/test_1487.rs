#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_iterator() {
        let mut expected = LiteMap::with_capacity(4);
        expected.insert(1, "updated-one");
        expected.insert(2, "original-two");
        expected.insert(3, "original-three");
        expected.insert(4, "updated-four");

        let actual = [
            (1, "original-one"),
            (2, "original-two"),
            (4, "original-four"),
            (4, "updated-four"),
            (1, "updated-one"),
            (3, "original-three"),
        ]
        .into_iter()
        .collect::<LiteMap<_, _>>();

        assert_eq!(expected, actual);
    }

    #[test]
    fn extend() {
        let mut expected: LiteMap<i32, &str> = LiteMap::with_capacity(4);
        expected.insert(1, "updated-one");
        expected.insert(2, "original-two");
        expected.insert(3, "original-three");
        expected.insert(4, "updated-four");

        let mut actual: LiteMap<i32, &str> = LiteMap::new();
        actual.insert(1, "original-one");
        actual.extend([
            (2, "original-two"),
            (4, "original-four"),
            (4, "updated-four"),
            (1, "updated-one"),
            (3, "original-three"),
        ]);

        assert_eq!(expected, actual);
    }

    #[test]
    fn extend2() {
        let mut map: LiteMap<usize, &str> = LiteMap::new();
        map.extend(make_13());
        map.extend(make_24());
        map.extend(make_24());
        map.extend(make_46());
        map.extend(make_13());
        map.extend(make_46());
        assert_eq!(map.len(), 5);
    }

    fn make_13() -> LiteMap<usize, &'static str> {
        let mut result = LiteMap::new();
        result.insert(1, "one");
        result.insert(3, "three");
        result
    }

    fn make_24() -> LiteMap<usize, &'static str> {
        let mut result = LiteMap::new();
        result.insert(2, "TWO");
        result.insert(4, "FOUR");
        result
    }

    fn make_46() -> LiteMap<usize, &'static str> {
        let mut result = LiteMap::new();
        result.insert(4, "four");
        result.insert(6, "six");
        result
    }

    #[test]
    fn extend_from_litemap_append() {
        let mut map = LiteMap::new();
        map.extend_from_litemap(make_13())
            .ok_or(())
            .expect_err("Append to empty map");
        map.extend_from_litemap(make_46())
            .ok_or(())
            .expect_err("Append to lesser map");
        assert_eq!(map.len(), 4);
    }

    #[test]
    fn extend_from_litemap_prepend() {
        let mut map = LiteMap::new();
        map.extend_from_litemap(make_46())
            .ok_or(())
            .expect_err("Prepend to empty map");
        map.extend_from_litemap(make_13())
            .ok_or(())
            .expect_err("Prepend to lesser map");
        assert_eq!(map.len(), 4);
    }

    #[test]
    fn extend_from_litemap_insert() {
        let mut map = LiteMap::new();
        map.extend_from_litemap(make_13())
            .ok_or(())
            .expect_err("Append to empty map");
        map.extend_from_litemap(make_24())
            .ok_or(())
            .expect_err("Insert with no conflict");
        map.extend_from_litemap(make_46())
            .ok_or(())
            .expect("Insert with conflict");
        assert_eq!(map.len(), 5);
    }

    #[test]
    fn test_const_cmp_bytes() {
        let strs = &["a", "aa", "abc", "abde", "bcd", "bcde"];
        for i in 0..strs.len() {
            for j in 0..strs.len() {
                let a = strs[i].as_bytes();
                let b = strs[j].as_bytes();
                assert_eq!(a.cmp(b), const_cmp_bytes(a, b));
            }
        }
    }

    #[test]
    fn into_iterator() {
        let mut map = LiteMap::<_, _, Vec<(_, _)>>::new();
        map.insert(4, "four");
        map.insert(6, "six");
        let mut reference = vec![(6, "six"), (4, "four")];

        for i in map {
            let r = reference.pop().unwrap();
            assert_eq!(r, i);
        }
        assert!(reference.is_empty());
    }

    #[test]
    fn entry_insert() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        assert!(matches!(map.entry(1), Entry::Vacant(_)));
        map.entry(1).or_insert("one");
        assert!(matches!(map.entry(1), Entry::Occupied(_)));
        assert_eq!(map.get(&1), Some(&"one"));
    }

    #[test]
    fn entry_insert_with() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        assert!(matches!(map.entry(1), Entry::Vacant(_)));
        map.entry(1).or_insert_with(|| "one");
        assert!(matches!(map.entry(1), Entry::Occupied(_)));
        assert_eq!(map.get(&1), Some(&"one"));
    }

    #[test]
    fn entry_vacant_insert() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        if let Entry::Vacant(entry) = map.entry(1) {
            entry.insert("one");
        }
        assert_eq!(map.get(&1), Some(&"one"));
    }

    #[test]
    fn entry_occupied_get_mut() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        map.insert(1, "one");
        if let Entry::Occupied(mut entry) = map.entry(1) {
            *entry.get_mut() = "uno";
        }
        assert_eq!(map.get(&1), Some(&"uno"));
    }

    #[test]
    fn entry_occupied_remove() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        map.insert(1, "one");
        if let Entry::Occupied(entry) = map.entry(1) {
            entry.remove();
        }
        assert_eq!(map.get(&1), None);
    }

    #[test]
    fn entry_occupied_key() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        map.insert(1, "one");
        if let Entry::Occupied(entry) = map.entry(1) {
            assert_eq!(entry.key(), &1);
        }
    }

    #[test]
    fn entry_occupied_get() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        map.insert(1, "one");
        if let Entry::Occupied(entry) = map.entry(1) {
            assert_eq!(entry.get(), &"one");
        }
    }

    #[test]
    fn entry_occupied_insert() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        map.insert(1, "one");
        if let Entry::Occupied(mut entry) = map.entry(1) {
            assert_eq!(entry.insert("uno"), "one");
        }
        assert_eq!(map.get(&1), Some(&"uno"));
    }

    #[test]
    fn entry_vacant_key() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        if let Entry::Vacant(entry) = map.entry(1) {
            assert_eq!(entry.key(), &1);
        }
    }

    #[test]
    fn entry_or_insert() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        map.entry(1).or_insert("one");
        assert_eq!(map.get(&1), Some(&"one"));
        map.entry(1).or_insert("uno");
        assert_eq!(map.get(&1), Some(&"one"));
    }

    #[test]
    fn entry_or_insert_with() {
        let mut map: LiteMap<i32, &str> = LiteMap::new();
        map.entry(1).or_insert_with(|| "one");
        assert_eq!(map.get(&1), Some(&"one"));
        map.entry(1).or_insert_with(|| "uno");
        assert_eq!(map.get(&1), Some(&"one"));
    }

    #[test]
    fn entry_or_default() {
        let mut map: LiteMap<i32, String> = LiteMap::new();
        map.entry(1).or_default();
        assert_eq!(map.get(&1), Some(&String::new()));
    }

    #[test]
    fn entry_and_modify() {
        let mut map: LiteMap<i32, i32> = LiteMap::new();
        map.entry(1).or_insert(10);
        map.entry(1).and_modify(|v| *v += 5);
        assert_eq!(map.get(&1), Some(&15));
        map.entry(2).and_modify(|v| *v += 5).or_insert(20);
        assert_eq!(map.get(&2), Some(&20));
    }
}
