#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_check_length() {
        let mut m: Map<String, i32, 10> = Map::new();
        assert_eq!(m.insert("first".to_string(), 42), None);
        assert_eq!(1, m.len());
        assert_eq!(m.insert("second".to_string(), 16), None);
        assert_eq!(2, m.len());
        assert_eq!(m.insert("first".to_string(), 16), Some(42));
        assert_eq!(2, m.len());
    }

    #[test]
    fn overwrites_keys() {
        let mut m: Map<i32, i32, 1> = Map::new();
        assert_eq!(m.insert(1, 42), None);
        assert_eq!(m.insert(1, 42), Some(42));
        assert_eq!(1, m.len());
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn cant_write_into_empty_map() {
        let mut m: Map<i32, i32, 0> = Map::new();
        assert_eq!(m.insert(1, 42), None);
    }

    #[test]
    fn empty_length() {
        let m: Map<u32, u32, 10> = Map::new();
        assert_eq!(0, m.len());
    }

    #[test]
    fn is_empty_check() {
        let mut m: Map<u32, u32, 10> = Map::new();
        assert!(m.is_empty());
        assert_eq!(m.insert(42, 42), None);
        assert!(!m.is_empty());
    }

    #[test]
    fn insert_and_gets() {
        let mut m: Map<String, i32, 10> = Map::new();
        assert_eq!(m.insert("one".to_string(), 42), None);
        assert_eq!(m.insert("two".to_string(), 16), None);
        assert_eq!(16, *m.get("two").unwrap());
    }

    #[test]
    fn insert_and_gets_mut() {
        let mut m: Map<i32, [i32; 3], 10> = Map::new();
        assert_eq!(m.insert(42, [1, 2, 3]), None);
        let a = m.get_mut(&42).unwrap();
        a[0] = 500;
        assert_eq!(500, m.get(&42).unwrap()[0]);
    }

    #[test]
    fn checks_key() {
        let mut m: Map<String, i32, 10> = Map::new();
        assert_eq!(m.insert("one".to_string(), 42), None);
        assert!(m.contains_key("one"));
        assert!(!m.contains_key("another"));
    }

    #[test]
    fn gets_missing_key() {
        let mut m: Map<String, i32, 10> = Map::new();
        assert_eq!(m.insert("one".to_string(), 42), None);
        assert!(m.get("two").is_none());
    }

    #[test]
    fn mut_gets_missing_key() {
        let mut m: Map<String, i32, 10> = Map::new();
        assert_eq!(m.insert("one".to_string(), 42), None);
        assert!(m.get_mut("two").is_none());
    }

    #[test]
    fn removes_simple_pair() {
        let mut m: Map<String, i32, 10> = Map::new();
        assert_eq!(m.insert("one".to_string(), 42), None);
        assert_eq!(m.remove("one"), Some(42));
        assert_eq!(m.remove("another"), None);
        assert!(m.get("one").is_none());
    }

    #[cfg(test)]
    #[derive(Clone, PartialEq, Debug)]
    struct Foo {
        v: [u32; 3],
    }

    #[test]
    fn insert_struct() {
        let mut m: Map<u8, Foo, 8> = Map::new();
        let foo = Foo { v: [1, 2, 100] };
        assert_eq!(m.insert(1, foo), None);
        assert_eq!(100, m.into_iter().next().unwrap().1.v[2]);
    }

    #[cfg(test)]
    #[derive(Clone, PartialEq, Debug)]
    struct Composite {
        r: Map<u8, u8, 1>,
    }

    #[test]
    fn insert_composite() {
        let mut m: Map<u8, Composite, 8> = Map::new();
        let c = Composite { r: Map::new() };
        assert_eq!(m.insert(1, c), None);
        assert_eq!(0, m.into_iter().next().unwrap().1.r.len());
    }

    #[test]
    fn large_map_in_heap() {
        let m: Box<Map<u64, [u64; 10], 10>> = Box::new(Map::new());
        assert_eq!(0, m.len());
    }

    #[test]
    fn clears_it_up() {
        use std::rc::Rc;
        let v = vec![1, 2, 3];
        let vrc = Rc::new(v);
        let mut m: Map<String, Rc<Vec<i32>>, 10> = Map::new();
        let vrc_1 = Rc::clone(&vrc);
        let vrc_2 = Rc::clone(&vrc);
        assert_eq!(m.insert("one".to_string(), vrc_1), None);
        assert_eq!(m.insert("two".to_string(), vrc_2), None);
        assert_eq!(Rc::strong_count(&vrc), 3);
        m.clear();
        assert_eq!(0, m.len());
        assert_eq!(Rc::strong_count(&vrc), 1);
    }

    #[test]
    fn retain_test() {
        let vec: Vec<(i32, i32)> = (0..8).map(|x| (x, x * 10)).collect();
        let mut m: Map<i32, i32, 10> = Map::from_iter(vec);
        assert_eq!(m.len(), 8);
        m.retain(|&k, _| k < 6);
        assert_eq!(m.len(), 6);
        m.retain(|_, &mut v| v > 30);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn retain_with_mutable_value() {
        let mut map: Map<&str, i32, 5> = Map::new();
        map.insert("key1", 10);
        map.insert("key2", 20);
        map.insert("key3", 30);
        // Retain only keys where the value is greater than 15, and double the retained values.
        map.retain(|_, value| {
            if *value > 15 {
                *value *= 2;
                true
            } else {
                false
            }
        });
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("key1"), None);
        assert_eq!(map.get("key2"), Some(&40));
        assert_eq!(map.get("key3"), Some(&60));
    }

    #[test]
    fn insert_many_and_remove() {
        let mut m: Map<usize, u64, 4> = Map::new();
        for _ in 0..2 {
            let cap = m.capacity();
            for i in 0..cap {
                assert_eq!(m.insert(i, 256), None);
                assert_eq!(m.remove(&i), Some(256));
            }
        }
    }

    #[test]
    fn get_key_value() {
        let mut m: Map<String, i32, 10> = Map::new();
        let k = "key".to_string();
        assert_eq!(m.insert(k.clone(), 42), None);
        assert_eq!(m.get_key_value(&k), Some((&k, &42)));
        assert!(m.contains_key(&k));
    }

    #[test]
    fn get_absent_key_value() {
        let mut m: Map<String, i32, 10> = Map::new();
        assert_eq!(m.insert("one".to_string(), 42), None);
        assert_eq!(m.get_key_value("two"), None);
    }

    #[test]
    fn remove_entry_present() {
        let mut m: Map<String, i32, 10> = Map::new();
        let k = "key".to_string();
        assert_eq!(m.insert(k.clone(), 42), None);
        assert_eq!(m.remove_entry(&k), Some((k.clone(), 42)));
        assert!(!m.contains_key(&k));
    }

    #[test]
    fn remove_entry_absent() {
        let mut m: Map<String, i32, 10> = Map::new();
        assert_eq!(m.insert("one".to_string(), 42), None);
        assert_eq!(m.remove_entry("two"), None);
    }

    #[test]
    fn drop_removed_entry() {
        use std::rc::Rc;
        let mut m: Map<(), Rc<()>, 8> = Map::new();
        let v = Rc::new(());
        assert_eq!(m.insert((), Rc::clone(&v)), None);
        assert_eq!(Rc::strong_count(&v), 2);
        assert_eq!(m.remove_entry(&()), Some(((), Rc::clone(&v))));
        assert_eq!(Rc::strong_count(&v), 1);
    }

    #[test]
    fn insert_after_remove() {
        let mut m: Map<_, _, 1> = Map::new();
        assert_eq!(m.insert(1, 2), None);
        assert_eq!(m.remove(&1), Some(2));
        assert_eq!(m.insert(1, 3), None);
    }

    #[test]
    fn insert_drop_duplicate() {
        use std::rc::Rc;
        let mut m: Map<_, _, 1> = Map::new();
        let v = Rc::new(());
        assert_eq!(m.insert((), Rc::clone(&v)), None);
        assert_eq!(Rc::strong_count(&v), 2);
        assert_eq!(m.insert((), Rc::clone(&v)), Some(Rc::clone(&v)));
        assert_eq!(Rc::strong_count(&v), 2);
    }

    #[test]
    fn insert_duplicate_after_remove() {
        let mut m: Map<_, _, 2> = Map::new();
        assert_eq!(m.insert(1, 1), None);
        assert_eq!(m.insert(2, 2), None);
        assert_eq!(m.remove(&1), Some(1));
        assert_eq!(m.insert(2, 3), Some(2));
        assert_eq!(1, m.len());
        assert_eq!(3, m[&2]);
    }

    #[test]
    fn insert_unchecked_inserts_new_key() {
        let mut m: Map<i32, i32, 3> = Map::new();
        unsafe {
            assert_eq!(m.insert_unchecked(1, 10), None);
            assert_eq!(m.get(&1), Some(&10));
        }
    }

    #[test]
    fn insert_unchecked_overwrites_existing_key() {
        let mut m: Map<i32, i32, 3> = Map::new();
        unsafe {
            assert_eq!(m.insert_unchecked(1, 10), None);
            assert_eq!(m.insert_unchecked(1, 20), Some(10));
            assert_eq!(m.get(&1), Some(&20));
        }
    }

    #[test]
    #[should_panic(expected = "No more key-value slot available in the map")]
    #[cfg(debug_assertions)]
    fn insert_unchecked_panics_when_map_is_full() {
        let mut m: Map<i32, i32, 1> = Map::new();
        unsafe {
            assert_eq!(m.insert_unchecked(1, 10), None);
            m.insert_unchecked(2, 20); // This should panic in debug mode.
        }
    }

    #[test]
    fn insert_unchecked_does_not_panic_in_release_mode() {
        let mut m: Map<i32, i32, 1> = Map::new();
        unsafe {
            assert_eq!(m.insert_unchecked(1, 10), None);
            // In release mode, this will cause undefined behavior if accessed.
            // m.insert_unchecked(2, 20);
        }
    }

    #[test]
    fn insert_unchecked_handles_duplicate_keys() {
        let mut m: Map<i32, i32, 2> = Map::new();
        unsafe {
            assert_eq!(m.insert_unchecked(1, 10), None);
            assert_eq!(m.insert_unchecked(1, 15), Some(10));
            assert_eq!(m.get(&1), Some(&15));
        }
    }

    #[test]
    fn insert_unchecked_with_struct_key() {
        #[derive(PartialEq, Debug)]
        struct Key {
            id: u32,
        }
        let mut m: Map<Key, i32, 2> = Map::new();
        let key1 = Key { id: 1 };
        let key2 = Key { id: 2 };
        unsafe {
            assert_eq!(m.insert_unchecked(key1, 42), None);
            assert_eq!(m.insert_unchecked(key2, 84), None);
            assert_eq!(m.get(&Key { id: 1 }), Some(&42));
            assert_eq!(m.get(&Key { id: 2 }), Some(&84));
        }
    }

    #[test]
    fn checked_insert_updates_existing_key() {
        let mut m: Map<String, i32, 10> = Map::new();
        assert_eq!(m.checked_insert("key".to_string(), 42), Some(None));
        assert_eq!(m.checked_insert("key".to_string(), 100), Some(Some(42)));
        assert_eq!(m.get("key"), Some(&100));
    }

    #[test]
    fn checked_insert_inserts_new_key_when_space_available() {
        let mut m: Map<String, i32, 2> = Map::new();
        assert_eq!(m.checked_insert("key1".to_string(), 42), Some(None));
        assert_eq!(m.checked_insert("key2".to_string(), 100), Some(None));
        assert_eq!(m.get("key1"), Some(&42));
        assert_eq!(m.get("key2"), Some(&100));
    }

    #[test]
    fn checked_insert_fails_when_map_is_full() {
        let mut m: Map<String, i32, 1> = Map::new();
        assert_eq!(m.checked_insert("key1".to_string(), 42), Some(None));
        assert_eq!(m.checked_insert("key2".to_string(), 100), None);
        assert_eq!(m.get("key1"), Some(&42));
        assert_eq!(m.get("key2"), None);
        assert_eq!(m.checked_insert("key1".to_string(), 43), Some(Some(42)));
        assert_eq!(m.get("key1"), Some(&43));
    }

    #[test]
    fn checked_insert_handles_empty_map() {
        let mut m: Map<String, i32, 0> = Map::new();
        assert_eq!(m.checked_insert("key".to_string(), 42), None);
        assert!(m.is_empty());
    }

    #[test]
    fn checked_insert_replaces_value_for_existing_key() {
        let mut m: Map<i32, i32, 3> = Map::new();
        assert_eq!(m.checked_insert(1, 10), Some(None));
        assert_eq!(m.checked_insert(1, 20), Some(Some(10)));
        assert_eq!(m.get(&1), Some(&20));
    }

    #[test]
    fn checked_insert_does_not_affect_other_keys() {
        let mut m: Map<i32, i32, 3> = Map::new();
        assert_eq!(m.checked_insert(1, 10), Some(None));
        assert_eq!(m.checked_insert(2, 20), Some(None));
        assert_eq!(m.checked_insert(1, 30), Some(Some(10)));
        assert_eq!(m.get(&1), Some(&30));
        assert_eq!(m.get(&2), Some(&20));
    }

    #[test]
    fn get_disjoint_mut_non_overlapping_keys() {
        let mut map: Map<&str, i32, 5> = Map::new();
        map.insert("key1", 10);
        map.insert("key2", 20);
        map.insert("key3", 30);
        map.insert("key4", 40);
        let [v3, v1, v2] = map.get_disjoint_mut(["key3", "key1", "key2"]);
        assert_eq!(v1, Some(&mut 10));
        assert_eq!(v2, Some(&mut 20));
        assert_eq!(v3, Some(&mut 30));
        if let Some(v1) = v1 {
            *v1 = 100;
        }
        if let Some(v2) = v2 {
            *v2 = 200;
        }
        if let Some(v3) = v3 {
            *v3 = 300;
        }
        assert_eq!(map.get("key1"), Some(&100));
        assert_eq!(map.get("key2"), Some(&200));
        assert_eq!(map.get("key3"), Some(&300));
        assert_eq!(map.get("key4"), Some(&40));
    }

    #[test]
    fn get_disjoint_unchecked_mut_non_overlapping_keys() {
        let mut map: Map<&str, i32, 5> = Map::new();
        map.insert("key1", 10);
        map.insert("key2", 20);
        map.insert("key3", 30);
        map.insert("key4", 40);
        let [v3, v1, v2] = unsafe { map.get_disjoint_unchecked_mut(["key3", "key1", "key2"]) };
        assert_eq!(v1, Some(&mut 10));
        assert_eq!(v2, Some(&mut 20));
        assert_eq!(v3, Some(&mut 30));
        if let Some(v1) = v1 {
            *v1 = 100;
        }
        if let Some(v2) = v2 {
            *v2 = 200;
        }
        if let Some(v3) = v3 {
            *v3 = 300;
        }
        assert_eq!(map.get("key1"), Some(&100));
        assert_eq!(map.get("key2"), Some(&200));
        assert_eq!(map.get("key3"), Some(&300));
        assert_eq!(map.get("key4"), Some(&40));
    }

    #[test]
    #[should_panic(expected = "Overlapping keys")]
    fn get_disjoint_mut_overlapping_keys() {
        let mut map: Map<&str, i32, 5> = Map::new();
        map.insert("key1", 10);
        map.insert("key2", 20);
        // This should panic because the keys overlap.
        let _ = map.get_disjoint_mut(["key1", "key1"]);
    }

    #[test]
    fn get_disjoint_mut_missing_keys() {
        let mut map: Map<&str, i32, 5> = Map::new();
        map.insert("key2", 20);
        let [v1, v2] = map.get_disjoint_mut(["key1", "key2"]);
        assert_eq!(v1, None);
        assert_eq!(v2, Some(&mut 20));
    }

    #[test]
    fn get_disjoint_unchecked_mut_missing_keys() {
        let mut map: Map<&str, i32, 5> = Map::new();
        map.insert("key2", 20);
        let [v1, v2] = unsafe { map.get_disjoint_unchecked_mut(["key1", "key2"]) };
        assert_eq!(v1, None);
        assert_eq!(v2, Some(&mut 20));
    }

    #[test]
    fn get_disjoint_mut_empty_keys() {
        let mut map: Map<&str, i32, 5> = Map::new();
        let result: [Option<&mut i32>; 0] = map.get_disjoint_mut::<&str, 0>([]);
        assert!(result.is_empty());
    }

    #[test]
    fn get_disjoint_unchecked_mut_empty_keys() {
        let mut map: Map<&str, i32, 5> = Map::new();
        let result: [Option<&mut i32>; 0] =
            unsafe { map.get_disjoint_unchecked_mut::<&str, 0>([]) };
        assert!(result.is_empty());
    }

    #[test]
    fn get_disjoint_mut_keys_more_than_capacity() {
        let mut map: Map<&str, i32, 3> = Map::new();
        assert!(map.is_empty());
        let [v1] = map.get_disjoint_mut(["key1"]);
        assert_eq!(v1, None);
        map.insert("key1", 10);
        let [v1] = map.get_disjoint_mut(["key1"]);
        assert_eq!(v1, Some(&mut 10));
        map.insert("key2", 20);
        map.insert("key3", 30);
        assert!(!map.is_empty());
        let result = map.get_disjoint_mut(["key1", "key2", "key3"]);
        result.iter().all(|x| x.is_some());
        let [v0, v1, v2, v3, v4] = map.get_disjoint_mut(["key0", "key1", "key2", "key3", "key4"]);
        assert_eq!(v0, None);
        assert_eq!(v1, Some(&mut 10));
        assert_eq!(v2, Some(&mut 20));
        assert_eq!(v3, Some(&mut 30));
        assert_eq!(v4, None);
    }

    mod book_example {
        use super::*;

        #[derive(Debug, Clone)]
        struct Book {
            isbn: u64,
            title: String,
        }

        impl PartialEq for Book {
            fn eq(&self, other: &Self) -> bool {
                self.isbn == other.isbn
            }
        }

        impl Eq for Book {}

        #[test]
        fn insert_will_not_update_the_key() {
            // library <book, inventory> map
            let mut library: Map<Book, u32, 5> = Map::new();
            // The book has old and new publish versions, but they are the same book.
            let csapp_old = Book {
                isbn: 9780134092669,
                title: "CSAPP 3rd".to_string(),
            };
            let csapp_new = Book {
                isbn: 9780134092669,
                title: "Computer Systems: A Programmer's Perspective (3rd Edition)".to_string(),
            };
            assert_ne!(csapp_old.title, csapp_new.title);
            assert_eq!(csapp_old, csapp_new);
            // Insert the old version of the book.
            assert_eq!(library.insert(csapp_old.clone(), 10), None);
            assert_eq!(library.get_key_value(&csapp_old), Some((&csapp_old, &10)));
            assert_eq!(library.len(), 1);
            // After some new editions of books have arrived.
            assert_eq!(library.insert(csapp_new.clone(), 20), Some(10));
            // the key is still `csapp_old` instead of `csapp_new`.
            assert_eq!(library.get_key_value(&csapp_new), Some((&csapp_old, &20)));
            assert_eq!(library.len(), 1);
        }

        #[test]
        fn insert_key_value_will_update_the_key() {
            // library <book, inventory> map
            let mut library: Map<Book, u32, 5> = Map::new();
            // The book has old and new publish versions, but they are the same book.
            let csapp_old = Book {
                isbn: 9780134092669,
                title: "CSAPP 3rd".to_string(),
            };
            let csapp_new = Book {
                isbn: 9780134092669,
                title: "Computer Systems: A Programmer's Perspective (3rd Edition)".to_string(),
            };
            assert_ne!(csapp_old.title, csapp_new.title);
            assert_eq!(csapp_old, csapp_new);
            // Insert the old version of the book.
            assert_eq!(library.insert_key_value(csapp_old.clone(), 10), None);
            assert_eq!(library.get_key_value(&csapp_old), Some((&csapp_old, &10)));
            assert_eq!(library.len(), 1);
            // After some new editions of books have arrived.
            assert_eq!(
                library.insert_key_value(csapp_new.clone(), 20),
                Some((csapp_old.clone(), 10))
            );
            // the key will be updated to `csapp_new`, the old key will be included in return pair.
            assert_eq!(library.get_key_value(&csapp_old), Some((&csapp_new, &20)));
            assert_eq!(library.len(), 1);
        }
    }
}
