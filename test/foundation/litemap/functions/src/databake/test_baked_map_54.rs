#[test]
fn test_baked_map() {
    // Const construction:
    test_bake!(
        LiteMap<usize, &str, &[(usize, &str)]>,
        const,
        crate::LiteMap::from_sorted_store_unchecked(
            &[
                (1usize, "one"),
                (2usize, "two"),
                (10usize, "ten"),
            ]
        ),
        litemap
    );
    // Non-const construction:
    test_bake!(
        LiteMap<usize, String, Vec<(usize, String)>>,
        crate::LiteMap::from_sorted_store_unchecked(
            alloc::vec![
                (1usize, "one".to_owned()),
                (2usize, "two".to_owned()),
                (10usize, "ten".to_owned()),
            ]
        ),
        litemap
    );
}
