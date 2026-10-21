#[cfg(test)]
mod tests {
    use super::*;
    use crate::varzerovec::{Index16, Index32, Index8, VarZeroVecFormat};
    use crate::VarZeroSlice;
    use crate::VarZeroVec;

    #[test]
    fn test_pairvarule_validate() {
        let vec: Vec<(&str, &[u8])> = vec![("a", b"b"), ("foo", b"bar"), ("lorem", b"ipsum\xFF")];
        let zerovec: VarZeroVec<Tuple2VarULE<str, [u8]>> = (&vec).into();
        let bytes = zerovec.as_bytes();
        let zerovec2 = VarZeroVec::parse_bytes(bytes).unwrap();
        assert_eq!(zerovec, zerovec2);

        // Test failed validation with a correctly sized but differently constrained tuple
        // Note: ipsum\xFF is not a valid str
        let zerovec3 = VarZeroVec::<Tuple2VarULE<str, str>>::parse_bytes(bytes);
        assert!(zerovec3.is_err());

        #[cfg(feature = "serde")]
        for val in zerovec.iter() {
            // Can't use inference due to https://github.com/rust-lang/rust/issues/130180
            test_utils::assert_serde_roundtrips::<Tuple2VarULE<str, [u8]>>(val);
        }
    }
    fn test_tripleule_validate_inner<Format: VarZeroVecFormat>() {
        let vec: Vec<(&str, &[u8], VarZeroVec<str>)> = vec![
            ("a", b"b", (&vec!["a", "b", "c"]).into()),
            ("foo", b"bar", (&vec!["baz", "quux"]).into()),
            (
                "lorem",
                b"ipsum\xFF",
                (&vec!["dolor", "sit", "amet"]).into(),
            ),
        ];
        let zerovec: VarZeroVec<Tuple3VarULE<str, [u8], VarZeroSlice<str>, Format>> = (&vec).into();
        let bytes = zerovec.as_bytes();
        let zerovec2 = VarZeroVec::parse_bytes(bytes).unwrap();
        assert_eq!(zerovec, zerovec2);

        // Test failed validation with a correctly sized but differently constrained tuple
        // Note: the str is unlikely to be a valid varzerovec
        let zerovec3 = VarZeroVec::<Tuple3VarULE<VarZeroSlice<str>, [u8], VarZeroSlice<str>, Format>>::parse_bytes(bytes);
        assert!(zerovec3.is_err());

        #[cfg(feature = "serde")]
        for val in zerovec.iter() {
            // Can't use inference due to https://github.com/rust-lang/rust/issues/130180
            test_utils::assert_serde_roundtrips::<Tuple3VarULE<str, [u8], VarZeroSlice<str>, Format>>(
                val,
            );
        }
    }

    #[test]
    fn test_tripleule_validate() {
        test_tripleule_validate_inner::<Index8>();
        test_tripleule_validate_inner::<Index16>();
        test_tripleule_validate_inner::<Index32>();
    }
}
