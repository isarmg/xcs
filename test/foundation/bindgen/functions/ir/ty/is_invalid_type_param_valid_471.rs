#[test]
fn is_invalid_type_param_valid() {
    let ty = Type::new(Some("foo".into()), None, TypeKind::TypeParam, false);
    assert!(!ty.is_invalid_type_param());
}
