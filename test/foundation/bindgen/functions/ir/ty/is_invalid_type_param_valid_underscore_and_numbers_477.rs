#[test]
fn is_invalid_type_param_valid_underscore_and_numbers() {
    let ty = Type::new(
        Some("_foo123456789_".into()),
        None,
        TypeKind::TypeParam,
        false,
    );
    assert!(!ty.is_invalid_type_param());
}
