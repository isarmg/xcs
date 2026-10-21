#[test]
fn test_layout_for_size() {
    use std::mem::size_of;
    let ptr_size = size_of::<*mut ()>();
    assert_eq!(
        Layout::for_size_internal(ptr_size, ptr_size),
        Layout::new(ptr_size, ptr_size)
    );
    assert_eq!(
        Layout::for_size_internal(ptr_size, 3 * ptr_size),
        Layout::new(3 * ptr_size, ptr_size)
    );
}
