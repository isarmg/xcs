#[test]
fn test_size() {
    use std::mem::size_of;

    assert_eq!(size_of::<OnceCell<bool>>(), 1 * size_of::<bool>() + size_of::<u8>());
}
