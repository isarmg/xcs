#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "unicode-case")]
    fn simple_fold_ok(c: char) -> impl Iterator<Item = char> {
        SimpleCaseFolder::new().unwrap().mapping(c).iter().copied()
    }

    #[cfg(feature = "unicode-case")]
    fn contains_case_map(start: char, end: char) -> bool {
        SimpleCaseFolder::new().unwrap().overlaps(start, end)
    }

    #[test]
    #[cfg(feature = "unicode-case")]
    fn simple_fold_k() {
        let xs: Vec<char> = simple_fold_ok('k').collect();
        assert_eq!(xs, alloc::vec!['K', 'K']);

        let xs: Vec<char> = simple_fold_ok('K').collect();
        assert_eq!(xs, alloc::vec!['k', 'K']);

        let xs: Vec<char> = simple_fold_ok('K').collect();
        assert_eq!(xs, alloc::vec!['K', 'k']);
    }

    #[test]
    #[cfg(feature = "unicode-case")]
    fn simple_fold_a() {
        let xs: Vec<char> = simple_fold_ok('a').collect();
        assert_eq!(xs, alloc::vec!['A']);

        let xs: Vec<char> = simple_fold_ok('A').collect();
        assert_eq!(xs, alloc::vec!['a']);
    }

    #[test]
    #[cfg(not(feature = "unicode-case"))]
    fn simple_fold_disabled() {
        assert!(SimpleCaseFolder::new().is_err());
    }

    #[test]
    #[cfg(feature = "unicode-case")]
    fn range_contains() {
        assert!(contains_case_map('A', 'A'));
        assert!(contains_case_map('Z', 'Z'));
        assert!(contains_case_map('A', 'Z'));
        assert!(contains_case_map('@', 'A'));
        assert!(contains_case_map('Z', '['));
        assert!(contains_case_map('☃', 'Ⰰ'));

        assert!(!contains_case_map('[', '['));
        assert!(!contains_case_map('[', '`'));

        assert!(!contains_case_map('☃', '☃'));
    }

    #[test]
    #[cfg(feature = "unicode-gencat")]
    fn regression_466() {
        use super::{CanonicalClassQuery, ClassQuery};

        let q = ClassQuery::OneLetter('C');
        assert_eq!(
            q.canonicalize().unwrap(),
            CanonicalClassQuery::GeneralCategory("Other")
        );
    }

    #[test]
    fn sym_normalize() {
        let sym_norm = symbolic_name_normalize;

        assert_eq!(sym_norm("Line_Break"), "linebreak");
        assert_eq!(sym_norm("Line-break"), "linebreak");
        assert_eq!(sym_norm("linebreak"), "linebreak");
        assert_eq!(sym_norm("BA"), "ba");
        assert_eq!(sym_norm("ba"), "ba");
        assert_eq!(sym_norm("Greek"), "greek");
        assert_eq!(sym_norm("isGreek"), "greek");
        assert_eq!(sym_norm("IS_Greek"), "greek");
        assert_eq!(sym_norm("isc"), "isc");
        assert_eq!(sym_norm("is c"), "isc");
        assert_eq!(sym_norm("is_c"), "isc");
    }

    #[test]
    fn valid_utf8_symbolic() {
        let mut x = b"abc\xFFxyz".to_vec();
        let y = symbolic_name_normalize_bytes(&mut x);
        assert_eq!(y, b"abcxyz");
    }
}
