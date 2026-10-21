#[cfg(test)]
mod tests {
    use super::Map;

    #[test]
    fn normal_drain() {
        let mut map = Map::<char, u8, 10>::from_iter([('a', 97), ('b', 98), ('c', 99), ('d', 100)]);
        let mut cloned_map = map.clone();
        let mut drain = map.drain();
        // For ExactSizeIterator
        assert_eq!(drain.len(), drain.size_hint().0);
        // Consume the first two items by iterator
        assert_eq!(drain.next(), Some(('a', 97)));
        assert_eq!(drain.next(), Some(('b', 98)));
        // We can fuse the drain
        let mut fuse_it = drain.fuse();
        assert_eq!(fuse_it.next(), Some(('c', 99)));
        assert_eq!(fuse_it.next(), Some(('d', 100)));
        // Further calls to next() should return None
        assert!(fuse_it.next().is_none());
        // Then fuse works. (It doesn't make sense in our Drain really, but it can.)
        assert!(fuse_it.next().is_none());
        let mut drain = cloned_map.drain();
        assert_eq!(drain.next(), Some(('a', 97)));
        // Three elements left for Drop
        drop(drain);
    }

    #[test]
    fn debug_trait_for_drain() {
        let mut map = Map::from([('x', 120), ('y', 121), ('z', 122)]);
        let drain = map.drain();
        // use Debug trait by `format!` macro
        let debug_output = format!("{:?}", drain);
        assert!(debug_output.contains("('x', 120)"));
        assert!(debug_output.contains("('y', 121)"));
        assert!(debug_output.contains("('z', 122)"));
    }
}
