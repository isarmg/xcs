#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fill_via_u32_chunks() {
        let src = [1, 2, 3];
        let mut dst = [0u8; 11];
        assert_eq!(fill_via_u32_chunks(&src, &mut dst), (3, 11));
        assert_eq!(dst, [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0]);

        let mut dst = [0u8; 13];
        assert_eq!(fill_via_u32_chunks(&src, &mut dst), (3, 12));
        assert_eq!(dst, [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0]);

        let mut dst = [0u8; 5];
        assert_eq!(fill_via_u32_chunks(&src, &mut dst), (2, 5));
        assert_eq!(dst, [1, 0, 0, 0, 2]);
    }

    #[test]
    fn test_fill_via_u64_chunks() {
        let src = [1, 2];
        let mut dst = [0u8; 11];
        assert_eq!(fill_via_u64_chunks(&src, &mut dst), (2, 11));
        assert_eq!(dst, [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0]);

        let mut dst = [0u8; 17];
        assert_eq!(fill_via_u64_chunks(&src, &mut dst), (2, 16));
        assert_eq!(dst, [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0]);

        let mut dst = [0u8; 5];
        assert_eq!(fill_via_u64_chunks(&src, &mut dst), (1, 5));
        assert_eq!(dst, [1, 0, 0, 0, 0]);
    }
}
