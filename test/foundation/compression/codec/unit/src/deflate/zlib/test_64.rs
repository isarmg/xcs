#[cfg(test)]
mod test {
    use crate::codec::shared::MZ_DEFAULT_WINDOW_BITS;
    #[test]
    fn zlib() {
        use super::super::*;
        use super::*;

        let test_level = |level, expected| {
            let flags = create_comp_flags_from_zip_params(
                level,
                MZ_DEFAULT_WINDOW_BITS,
                CompressionStrategy::Default as i32,
            );
            assert_eq!(zlib_level_from_flags(flags), expected);
        };

        assert_eq!(zlib_level_from_flags(DEFAULT_FLAGS), 2);
        test_level(0, 0);
        test_level(1, 0);
        test_level(2, 1);
        test_level(3, 1);
        for i in 4..=8 {
            test_level(i, 2)
        }
        test_level(9, 3);
        test_level(10, 3);
    }

    #[test]
    fn test_header() {
        let header = super::header_from_level(3, 8);
        assert_eq!(
            ((usize::from(header[0]) * 256) + usize::from(header[1])) % 31,
            0
        );
    }
}
