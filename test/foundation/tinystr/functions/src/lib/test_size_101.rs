#[test]
fn test_size() {
    assert_eq!(size_of::<TinyStr4>(), size_of::<Option<TinyStr4>>());
    assert_eq!(size_of::<TinyStr8>(), size_of::<Option<TinyStr8>>());
}
