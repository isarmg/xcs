#[test]
fn assert_core_type_id_small() {
    assert!(core::mem::size_of::<CoreTypeId>() <= 4);
}
