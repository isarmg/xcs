#[test]
fn assert_defined_type_small() {
    assert!(core::mem::size_of::<ComponentDefinedTypeId>() <= 8);
}
