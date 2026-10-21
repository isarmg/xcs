#[cfg(test)]
mod tests {
    use super::Set;

    #[test]
    fn test_sub_with_non_overlapping_sets() {
        let a = Set::from([1, 2, 3]);
        let b = Set::from([4, 5, 6]);
        let result = &a - &b;
        let expected = [1, 2, 3];
        let mut i = 0;
        for x in &result {
            assert!(expected.contains(x));
            i += 1;
        }
        assert_eq!(i, expected.len());
    }

    #[test]
    fn test_sub_with_overlapping_sets() {
        let a = Set::from([1, 2, 3, 4]);
        let b = Set::from([3, 4, 5]);
        let result = &a - &b;
        let expected = [1, 2];
        assert_eq!(
            expected.len(),
            result.iter().fold(0, |acc, x| {
                assert!(expected.contains(x));
                acc + 1
            })
        );
    }

    #[test]
    fn test_sub_with_empty_rhs() {
        let a = Set::from([1, 2, 3]);
        let b = Set::from([]);
        let result = &a - &b;
        let expected = Set::from([1, 2, 3, 3, 2, 1]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_sub_with_empty_lhs() {
        let a = Set::from([]);
        let b = Set::from([1, 2, 3]);
        let result = &a - &b;
        assert!(result.is_empty());
    }

    #[test]
    fn test_sub_with_identical_sets() {
        let a = Set::from([1, 2, 3]);
        let b = Set::from([1, 2, 3]);
        let result = &a - &b;
        assert!(result.is_empty());
    }
}
