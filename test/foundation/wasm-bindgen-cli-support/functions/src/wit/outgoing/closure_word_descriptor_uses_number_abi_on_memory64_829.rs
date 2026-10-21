#[test]
fn closure_word_descriptor_uses_number_abi_on_memory64() {
    assert_eq!(closure_word_descriptor(true), Descriptor::I64AsF64);
    assert_eq!(closure_word_descriptor(false), Descriptor::I32);
}
