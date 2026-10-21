#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(clippy::get_first)]
    fn test_new_single_const() {
        const MY_CONST_SLICE: ShortBoxSlice<i32> = ShortBoxSlice::new_single(42);

        assert_eq!(MY_CONST_SLICE.len(), 1);
        assert_eq!(MY_CONST_SLICE.get(0), Some(&42));
    }

    #[test]
    #[expect(clippy::redundant_pattern_matching)]
    fn test_get_single() {
        let mut vec = ShortBoxSlice::new();
        assert!(matches!(vec.single(), None));

        vec.push(100);
        assert!(matches!(vec.single(), Some(_)));

        vec.push(200);
        assert!(matches!(vec.single(), None));
    }
}
