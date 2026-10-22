#[cfg(test)]
mod tests {
    use super::Verdict;
    use test_case::test_case;

    #[test_case(Verdict::Admits, Verdict::Admits, Verdict::Admits)]
    #[test_case(Verdict::Admits, Verdict::Unknown, Verdict::Unknown)]
    #[test_case(Verdict::Unknown, Verdict::Admits, Verdict::Unknown)]
    #[test_case(Verdict::Unknown, Verdict::Unknown, Verdict::Unknown)]
    #[test_case(Verdict::Rejects, Verdict::Admits, Verdict::Rejects)]
    #[test_case(Verdict::Rejects, Verdict::Unknown, Verdict::Rejects)]
    #[test_case(Verdict::Admits, Verdict::Rejects, Verdict::Rejects)]
    #[test_case(Verdict::Unknown, Verdict::Rejects, Verdict::Rejects)]
    #[test_case(Verdict::Rejects, Verdict::Rejects, Verdict::Rejects)]
    fn and_truth_table(left: Verdict, right: Verdict, expected: Verdict) {
        assert_eq!(left.and(right), expected);
    }

    #[test_case(true, Verdict::Admits)]
    #[test_case(false, Verdict::Rejects)]
    fn from_bool_maps(admitted: bool, expected: Verdict) {
        assert_eq!(Verdict::from_bool(admitted), expected);
    }

    #[test_case(vec![], Verdict::Admits)]
    #[test_case(vec![Verdict::Admits, Verdict::Admits], Verdict::Admits)]
    #[test_case(vec![Verdict::Admits, Verdict::Unknown], Verdict::Unknown)]
    #[test_case(vec![Verdict::Unknown, Verdict::Rejects], Verdict::Rejects)]
    #[test_case(vec![Verdict::Rejects, Verdict::Unknown], Verdict::Rejects)]
    fn all_folds(verdicts: Vec<Verdict>, expected: Verdict) {
        assert_eq!(Verdict::all(verdicts), expected);
    }

    #[test_case(vec![], Verdict::Rejects)]
    #[test_case(vec![Verdict::Rejects, Verdict::Rejects], Verdict::Rejects)]
    #[test_case(vec![Verdict::Rejects, Verdict::Unknown], Verdict::Unknown)]
    #[test_case(vec![Verdict::Unknown, Verdict::Admits], Verdict::Admits)]
    #[test_case(vec![Verdict::Admits, Verdict::Rejects], Verdict::Admits)]
    fn any_folds(verdicts: Vec<Verdict>, expected: Verdict) {
        assert_eq!(Verdict::any(verdicts), expected);
    }
}
