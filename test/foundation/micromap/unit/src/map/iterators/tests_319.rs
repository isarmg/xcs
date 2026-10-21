#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_iterator() {
        let m: Map<u32, u32, 4> = Map::new();
        assert!(m.into_iter().next().is_none());
    }

    #[test]
    fn insert_and_jump_over_next() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("foo".to_string(), 42);
        let mut iter = m.into_iter();
        assert_eq!(42, iter.next().unwrap().1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn insert_and_iterate() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 42);
        m.insert("two".to_string(), 16);
        let mut sum = 0;
        for (_k, v) in m.iter() {
            sum += v;
        }
        assert_eq!(58, sum);
    }

    #[test]
    fn insert_and_into_iterate() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 42);
        m.insert("two".to_string(), 16);
        let mut sum = 0;
        for p in &m {
            sum += p.1;
        }
        assert_eq!(58, sum);
    }

    #[test]
    fn iterate_with_blanks() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 1);
        m.insert("two".to_string(), 3);
        m.insert("three".to_string(), 5);
        m.remove("two");
        let mut sum = 0;
        for (_k, v) in m.iter() {
            sum += v;
        }
        assert_eq!(6, sum);
    }

    #[test]
    fn debug_trait_for_iter() {
        let map = Map::from([('x', 120), ('y', 121), ('z', 122)]);
        let it = map.iter();
        // use Debug trait by `format!` macro
        let debug_output = format!("{:?}", it);
        assert!(debug_output.contains("('x', 120)"));
        assert!(debug_output.contains("('y', 121)"));
        assert!(debug_output.contains("('z', 122)"));
    }

    #[test]
    fn debug_trait_for_iter_mut() {
        let mut map = Map::from([('x', 120), ('y', 121), ('z', 122)]);
        let it = map.iter_mut();
        // use Debug trait by `format!` macro
        let debug_output = format!("{:?}", it);
        assert!(debug_output.contains("('x', 120)"));
        assert!(debug_output.contains("('y', 121)"));
        assert!(debug_output.contains("('z', 122)"));
    }

    #[test]
    fn debug_trait_for_into_iter() {
        let map = Map::from([('x', 120), ('y', 121), ('z', 122)]);
        let it = map.into_iter();
        // use Debug trait by `format!` macro
        let debug_output = format!("{:?}", it);
        assert!(debug_output.contains("('x', 120)"));
        assert!(debug_output.contains("('y', 121)"));
        assert!(debug_output.contains("('z', 122)"));
    }

    #[test]
    fn into_iterate_with_blanks() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 1);
        m.insert("two".to_string(), 3);
        m.insert("three".to_string(), 5);
        m.remove("two");
        let mut sum = 0;
        for (_k, v) in m {
            sum += v;
        }
        assert_eq!(6, sum);
    }

    #[test]
    fn change_with_iter_mut() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 2);
        m.insert("two".to_string(), 3);
        m.insert("three".to_string(), 5);
        for (_k, v) in m.iter_mut() {
            *v *= 2;
        }
        let sum = m.iter().map(|p| p.1).sum::<i32>();
        assert_eq!(20, sum);
    }

    #[test]
    fn iter_mut_with_blanks() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 1);
        m.insert("two".to_string(), 3);
        m.insert("three".to_string(), 5);
        assert_eq!(m.iter_mut().count(), 3);
        m.remove("two");
        assert_eq!(m.iter_mut().count(), 2);
        assert_eq!(m.iter_mut().last().unwrap().1, &5);
    }

    #[test]
    fn iter_mut_key_is_not_mut() {
        let mut m: Map<u8, i32, 10> = Map::new();
        m.insert(1, 10);
        m.insert(2, 20);
        for (k, v) in m.iter_mut() {
            // *k += 1; // this will not compile, because `k` is &u8
            *v += *k as i32; // v is &mut i32
        }
    }

    #[test]
    fn into_iter_mut() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 2);
        m.insert("two".to_string(), 3);
        m.insert("three".to_string(), 5);
        for (_k, v) in &mut m {
            *v *= 2;
        }
        let sum = m.iter().map(|p| p.1).sum::<i32>();
        assert_eq!(20, sum);
    }

    #[test]
    fn into_iter_drop() {
        use std::rc::Rc;
        let mut m: Map<i32, Rc<()>, 8> = Map::new();
        let v = Rc::new(());
        let n = 8;
        for i in 0..n {
            m.insert(i, Rc::clone(&v));
        }
        assert_eq!(Rc::strong_count(&v), (n + 1) as usize);
        let _p = m.into_iter().nth(3);
        assert_eq!(Rc::strong_count(&v), 2); // v & p
    }

    #[test]
    fn iter_size_hint() {
        let mut m: Map<char, u32, 4> = Map::new();
        m.insert('a', 97);
        m.insert('c', 99);
        let it = m.iter();
        assert_eq!(it.len(), 2);
        let mut it_mut = m.iter_mut();
        assert!(it_mut.next().is_some());
        assert_eq!(it_mut.len(), 1);
        assert_eq!(it_mut.len(), it_mut.size_hint().0);
        let mut it_into = m.into_iter();
        assert!(it_into.next().is_some());
        assert!(it_into.next().is_some());
        assert!(it_into.next().is_none());
        assert!(it_into.next().is_none());
        assert_eq!(it_into.len(), 0);
    }

    #[test]
    fn iter_series_default() {
        let _i = Iter::<String, u32>::default();
        let _i = IterMut::<String, u32>::default();
        let _i = IntoIter::<String, u32, 3>::default();
    }

    #[test]
    fn iter_count() {
        let mut m: Map<char, u32, 4> = Map::new();
        m.insert('a', 97);
        m.insert('b', 98);
        m.insert('c', 99);
        let it = m.iter();
        assert_eq!(it.count(), 3); // Count all elements
    }

    #[test]
    fn iter_mut_count() {
        let mut m: Map<char, u32, 4> = Map::new();
        m.insert('a', 97);
        m.insert('b', 98);
        m.insert('c', 99);
        let it_mut = m.iter_mut();
        assert_eq!(it_mut.count(), 3); // Count all elements
    }

    #[test]
    fn into_iter_count() {
        let mut m: Map<char, u32, 4> = Map::new();
        m.insert('a', 97);
        m.insert('b', 98);
        m.insert('c', 99);
        let it_owned = m.into_iter();
        assert_eq!(it_owned.count(), 3); // Count all elements
    }
}
