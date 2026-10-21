#[test]
fn vector_kind_accepts_memory64_scalar_descriptors() {
    assert_eq!(
        Descriptor::Vector(Box::new(Descriptor::U64AsF64)).vector_kind(),
        Some(VectorKind::U64)
    );
    assert_eq!(
        Descriptor::Ref(Box::new(Descriptor::Slice(Box::new(Descriptor::I64AsF64)))).vector_kind(),
        Some(VectorKind::I64)
    );
}
