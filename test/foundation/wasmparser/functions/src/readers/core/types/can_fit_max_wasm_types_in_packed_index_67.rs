#[test]
fn can_fit_max_wasm_types_in_packed_index() {
    assert!(PackedIndex::can_represent_index(
        crate::limits::MAX_WASM_TYPES as u32
    ));
    assert!(PackedIndex::can_represent_index(
        0b00000000_00001111_00000000_00000000
    ));
    assert!(PackedIndex::can_represent_index(
        0b00000000_00000000_11111111_00000000
    ));
    assert!(PackedIndex::can_represent_index(
        0b00000000_00000000_00000000_11111111
    ));
    assert!(PackedIndex::can_represent_index(0));
}
