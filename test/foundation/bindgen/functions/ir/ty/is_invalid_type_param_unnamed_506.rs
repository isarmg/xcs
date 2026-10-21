#[test]
#[should_panic(expected = "Unnamed named type")]
fn is_invalid_type_param_unnamed() {
    let ty = Type::new(None, None, TypeKind::TypeParam, false);
    assert!(ty.is_invalid_type_param());
}
