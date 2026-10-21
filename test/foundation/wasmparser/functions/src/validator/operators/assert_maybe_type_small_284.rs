#[test]
fn assert_maybe_type_small() {
    assert!(core::mem::size_of::<MaybeType>() == 8);
}
