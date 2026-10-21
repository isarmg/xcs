#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_usize_from_u64_saturated() {
        const USIZE_MAX: u64 = u64_from_usize(usize::MAX);
        assert_eq!(usize_from_u64_saturated(u64::MIN), usize::MIN);
        assert_eq!(usize_from_u64_saturated(USIZE_MAX), usize::MAX);
        assert_eq!(usize_from_u64_saturated(USIZE_MAX - 1), usize::MAX - 1);

        #[cfg(not(target_pointer_width = "64"))]
        {
            assert_eq!(usize_from_u64_saturated(USIZE_MAX + 1), usize::MAX);
        }
    }
}
