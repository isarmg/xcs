#[test]
fn is_invalid_type_param_valid_unnamed_kind() {
    let ty = Type::new(Some("foo".into()), None, TypeKind::Void, false);
    assert!(!ty.is_invalid_type_param());
}
