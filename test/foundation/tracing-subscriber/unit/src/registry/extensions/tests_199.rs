#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct MyType(i32);

    #[test]
    fn test_extensions() {
        let mut extensions = ExtensionsInner::new();

        extensions.insert(5i32);
        extensions.insert(MyType(10));

        assert_eq!(extensions.get(), Some(&5i32));
        assert_eq!(extensions.get_mut(), Some(&mut 5i32));

        assert_eq!(extensions.remove::<i32>(), Some(5i32));
        assert!(extensions.get::<i32>().is_none());

        assert_eq!(extensions.get::<bool>(), None);
        assert_eq!(extensions.get(), Some(&MyType(10)));
    }

    #[test]
    fn clear_retains_capacity() {
        let mut extensions = ExtensionsInner::new();
        extensions.insert(5i32);
        extensions.insert(MyType(10));
        extensions.insert(true);

        assert_eq!(extensions.map.len(), 3);
        let prev_capacity = extensions.map.capacity();
        extensions.clear();

        assert_eq!(
            extensions.map.len(),
            0,
            "after clear(), extensions map should have length 0"
        );
        assert_eq!(
            extensions.map.capacity(),
            prev_capacity,
            "after clear(), extensions map should retain prior capacity"
        );
    }

    #[test]
    fn clear_drops_elements() {
        use std::sync::Arc;
        struct DropMePlease(Arc<()>);
        struct DropMeTooPlease(Arc<()>);

        let mut extensions = ExtensionsInner::new();
        let val1 = DropMePlease(Arc::new(()));
        let val2 = DropMeTooPlease(Arc::new(()));

        let val1_dropped = Arc::downgrade(&val1.0);
        let val2_dropped = Arc::downgrade(&val2.0);
        extensions.insert(val1);
        extensions.insert(val2);

        assert!(val1_dropped.upgrade().is_some());
        assert!(val2_dropped.upgrade().is_some());

        extensions.clear();
        assert!(
            val1_dropped.upgrade().is_none(),
            "after clear(), val1 should be dropped"
        );
        assert!(
            val2_dropped.upgrade().is_none(),
            "after clear(), val2 should be dropped"
        );
    }
}
