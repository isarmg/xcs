#[cfg(all(test, feature = "suggestions"))]
mod test {
    use super::*;

    #[test]
    fn missing_letter() {
        let p_vals = ["test", "possible", "values"];
        assert_eq!(did_you_mean("tst", p_vals.iter()), vec!["test"]);
    }

    #[test]
    fn ambiguous() {
        let p_vals = ["test", "temp", "possible", "values"];
        assert_eq!(did_you_mean("te", p_vals.iter()), vec!["test", "temp"]);
    }

    #[test]
    fn unrelated() {
        let p_vals = ["test", "possible", "values"];
        assert_eq!(
            did_you_mean("hahaahahah", p_vals.iter()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn best_fit() {
        let p_vals = [
            "test",
            "possible",
            "values",
            "alignmentStart",
            "alignmentScore",
        ];
        assert_eq!(
            did_you_mean("alignmentScorr", p_vals.iter()),
            vec!["alignmentStart", "alignmentScore"]
        );
    }

    #[test]
    fn best_fit_long_common_prefix_issue_4660() {
        let p_vals = ["alignmentScore", "alignmentStart"];
        assert_eq!(
            did_you_mean("alignmentScorr", p_vals.iter()),
            vec!["alignmentStart", "alignmentScore"]
        );
    }

    #[test]
    fn flag_missing_letter() {
        let p_vals = ["test", "possible", "values"];
        assert_eq!(
            did_you_mean_flag("tst", &[], p_vals.iter(), []),
            Some(("test".to_owned(), None))
        );
    }

    #[test]
    fn flag_ambiguous() {
        let p_vals = ["test", "temp", "possible", "values"];
        assert_eq!(
            did_you_mean_flag("te", &[], p_vals.iter(), []),
            Some(("temp".to_owned(), None))
        );
    }

    #[test]
    fn flag_unrelated() {
        let p_vals = ["test", "possible", "values"];
        assert_eq!(
            did_you_mean_flag("hahaahahah", &[], p_vals.iter(), []),
            None
        );
    }

    #[test]
    fn flag_best_fit() {
        let p_vals = [
            "test",
            "possible",
            "values",
            "alignmentStart",
            "alignmentScore",
        ];
        assert_eq!(
            did_you_mean_flag("alignmentScorr", &[], p_vals.iter(), []),
            Some(("alignmentScore".to_owned(), None))
        );
    }
}
