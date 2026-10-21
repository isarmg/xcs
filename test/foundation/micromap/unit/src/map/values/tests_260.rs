#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterate_values() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 42);
        m.insert("two".to_string(), 16);
        let it = m.values();
        assert_eq!(it.len(), 2);
        assert_eq!(58, it.sum::<i32>());
    }

    #[test]
    fn iterate_values_mut() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 42);
        m.insert("two".to_string(), 16);
        let it_mut = m.values_mut();
        assert_eq!(it_mut.len(), 2);
        assert_eq!(it_mut.len(), it_mut.size_hint().0);
        assert_eq!(it_mut.len(), it_mut.size_hint().1.unwrap());
        it_mut.for_each(|v| *v *= 2);
        assert_eq!(116, m.values().sum::<i32>());
    }

    #[test]
    fn iterate_values_with_blanks() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 1);
        m.insert("two".to_string(), 3);
        m.insert("three".to_string(), 5);
        m.remove("two");
        assert_eq!(m.values().collect::<Vec<_>>(), [&1, &5]);
    }

    #[test]
    fn into_values_drop() {
        use std::rc::Rc;
        let mut m: Map<i32, Rc<()>, 8> = Map::new();
        let v = Rc::new(());
        for i in 0..8 {
            m.insert(i, Rc::clone(&v));
        }
        assert_eq!(9, Rc::strong_count(&v));
        let mut values = m.into_values();
        assert!(values.next().is_some());
        assert_eq!(values.len(), 7);
        assert!(values.next().is_some());
        assert_eq!(values.len(), values.size_hint().0);
        drop(values);
        assert_eq!(1, Rc::strong_count(&v));
    }

    #[test]
    fn values_clone() {
        let mut m: Map<i32, String, 10> = Map::new();
        m.insert(1, "foo".to_string());
        m.insert(2, "bar".to_string());
        let values = m.values();
        let cloned_values = values.clone();
        assert_eq!(
            values.collect::<Vec<_>>(),
            cloned_values.collect::<Vec<_>>()
        );
    }

    #[test]
    fn values_default() {
        let values: Values<'_, String, i32> = Values::default();
        assert_eq!(values.len(), 0);
        assert_eq!(values.size_hint(), (0, Some(0)));
    }

    #[test]
    fn values_mut_default() {
        let values_mut: ValuesMut<'_, String, i32> = ValuesMut::default();
        assert_eq!(values_mut.len(), 0);
        assert_eq!(values_mut.size_hint(), (0, Some(0)));
    }

    #[test]
    fn into_values_default() {
        let into_values: IntoValues<String, i32, 10> = IntoValues::default();
        assert_eq!(into_values.len(), 0);
        assert_eq!(into_values.size_hint(), (0, Some(0)));
    }

    #[test]
    fn values_debug() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 42);
        m.insert("two".to_string(), 16);
        let values = m.values();
        let debug_str = format!("{:?}", values);
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("16"));
    }

    #[test]
    fn values_mut_debug() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 42);
        m.insert("two".to_string(), 16);
        let values_mut = m.values_mut();
        let debug_str = format!("{:?}", values_mut);
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("16"));
    }

    #[test]
    fn into_values_debug() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 42);
        m.insert("two".to_string(), 16);
        let into_values = m.into_values();
        let debug_str = format!("{:?}", into_values);
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("16"));
    }
}
