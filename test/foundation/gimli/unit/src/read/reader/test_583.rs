#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_min_tombstone() {
        assert_eq!(u64::min_tombstone(1), 0xfe);
        assert_eq!(u64::min_tombstone(2), 0xfffe);
        assert_eq!(u64::min_tombstone(4), 0xffff_fffe);
        assert_eq!(u64::min_tombstone(8), 0xffff_ffff_ffff_fffe);
    }
}
