#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_zerovec_binary_search_in_range() {
        let zv: ZeroVec<u16> = ZeroVec::from_slice_or_alloc(&[11, 22, 33, 44, 55, 66, 77]);

        // Full range search
        assert_eq!(zv.zvl_binary_search_in_range(&11, 0..7), Some(Ok(0)));
        assert_eq!(zv.zvl_binary_search_in_range(&12, 0..7), Some(Err(1)));
        assert_eq!(zv.zvl_binary_search_in_range(&44, 0..7), Some(Ok(3)));
        assert_eq!(zv.zvl_binary_search_in_range(&45, 0..7), Some(Err(4)));
        assert_eq!(zv.zvl_binary_search_in_range(&77, 0..7), Some(Ok(6)));
        assert_eq!(zv.zvl_binary_search_in_range(&78, 0..7), Some(Err(7)));

        // Out-of-range search
        assert_eq!(zv.zvl_binary_search_in_range(&44, 0..2), Some(Err(2)));
        assert_eq!(zv.zvl_binary_search_in_range(&44, 5..7), Some(Err(0)));

        // Offset search
        assert_eq!(zv.zvl_binary_search_in_range(&44, 2..5), Some(Ok(1)));
        assert_eq!(zv.zvl_binary_search_in_range(&45, 2..5), Some(Err(2)));

        // Out-of-bounds
        assert_eq!(zv.zvl_binary_search_in_range(&44, 0..100), None);
        assert_eq!(zv.zvl_binary_search_in_range(&44, 100..200), None);
    }

    #[test]
    fn test_permute() {
        let mut zv: ZeroVec<u16> = ZeroVec::from_slice_or_alloc(&[11, 22, 33, 44, 55, 66, 77]);
        let mut permutation = vec![3, 2, 1, 0, 6, 5, 4];
        zv.zvl_permute(&mut permutation);
        assert_eq!(&zv, &[44, 33, 22, 11, 77, 66, 55]);

        let mut vzv: VarZeroVec<str> = VarZeroVec::from(
            VarZeroVecOwned::try_from_elements(&["11", "22", "33", "44", "55", "66", "77"])
                .unwrap(),
        );
        let mut permutation = vec![3, 2, 1, 0, 6, 5, 4];
        vzv.zvl_permute(&mut permutation);
        assert_eq!(&vzv, &["44", "33", "22", "11", "77", "66", "55"]);
    }
}
