#[test]
fn is_invalid_type_param_empty_name() {
    let ty = Type::new(Some(String::new()), None, TypeKind::TypeParam, false);
    assert!(ty.is_invalid_type_param());
}
