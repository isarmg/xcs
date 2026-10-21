#[test]
fn can_fit_max_wasm_types_in_ref_type() {
    fn can_roundtrip_index(index: u32) -> bool {
        assert!(RefType::can_represent_type_index(index));
        let rt = RefType::concrete(true, PackedIndex::from_module_index(index).unwrap());
        assert!(rt.is_nullable());
        let actual_index = match rt.type_index() {
            Some(i) => i,
            None => panic!(),
        };
        actual_index.as_module_index() == Some(index)
    }

    assert!(can_roundtrip_index(crate::limits::MAX_WASM_TYPES as u32));
    assert!(can_roundtrip_index(0b00000000_00001111_00000000_00000000));
    assert!(can_roundtrip_index(0b00000000_00000000_11111111_00000000));
    assert!(can_roundtrip_index(0b00000000_00000000_00000000_11111111));
    assert!(can_roundtrip_index(0));
}
