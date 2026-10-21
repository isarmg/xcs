#[test]
fn test_simple() {
    let var_tuple = VarTuple {
        sized: 1500u16,
        variable: "hello",
    };
    let var_tuple_ule = super::encode_varule_to_box(&var_tuple);
    assert_eq!(var_tuple_ule.sized.as_unsigned_int(), 1500);
    assert_eq!(&var_tuple_ule.variable, "hello");

    // Can't use inference due to https://github.com/rust-lang/rust/issues/130180
    #[cfg(feature = "serialization")]
    crate::ule::test_utils::assert_serialization_roundtrips::<VarTupleULE<u16, str>>(&var_tuple_ule);
}
