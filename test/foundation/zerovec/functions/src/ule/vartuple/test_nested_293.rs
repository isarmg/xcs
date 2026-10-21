#[test]
fn test_nested() {
    use crate::{ZeroSlice, ZeroVec};
    let var_tuple = VarTuple {
        sized: 2000u16,
        variable: VarTuple {
            sized: '🦙',
            variable: ZeroVec::alloc_from_slice(b"ICU"),
        },
    };
    let var_tuple_ule = super::encode_varule_to_box(&var_tuple);
    assert_eq!(var_tuple_ule.sized.as_unsigned_int(), 2000u16);
    assert_eq!(var_tuple_ule.variable.sized.to_char(), '🦙');
    assert_eq!(
        &var_tuple_ule.variable.variable,
        ZeroSlice::from_ule_slice(b"ICU")
    );
    // Can't use inference due to https://github.com/rust-lang/rust/issues/130180
    #[cfg(feature = "serialization")]
    crate::ule::test_utils::assert_serialization_roundtrips::<
        VarTupleULE<u16, VarTupleULE<char, ZeroSlice<_>>>,
    >(&var_tuple_ule);
}
