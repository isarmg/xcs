#[test]
fn weird_empty_representation_equality() {
    assert_eq!(
        VarZeroVec::<str>::parse_bytes(&[0, 0, 0, 0]).unwrap(),
        VarZeroVec::<str>::parse_bytes(&[]).unwrap()
    );
}
