#[test]
fn is_invalid_type_param_invalid_remaining() {
    let ty = Type::new(Some("foo-".into()), None, TypeKind::TypeParam, false);
    assert!(ty.is_invalid_type_param());
}
