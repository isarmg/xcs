#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn custom_boundary_inequality() {
        // Custom boundaries are never equal because they contain function pointers
        let a = Boundary::Custom {
            condition: |_| true,
            start: 0,
            len: 0,
        };
        let b = a;

        assert_ne!(a, b)
    }

    #[test]
    fn default_boundary_equality() {
        assert_eq!(Boundary::Hyphen, Boundary::Hyphen);
        assert_eq!(Boundary::Space, Boundary::Space);
        assert_ne!(Boundary::Hyphen, Boundary::Space);
    }

    #[rstest]
    #[case(Boundary::Hyphen, "a-b-c", vec!["a", "b", "c"])]
    #[case(Boundary::Underscore, "a_b_c", vec!["a", "b", "c"])]
    #[case(Boundary::Space, "a b c", vec!["a", "b", "c"])]
    #[case(Boundary::LowerUpper, "lowerUpperUpper", vec!["lower", "Upper", "Upper"])]
    #[case(Boundary::UpperLower, "ABc", vec!["AB", "c"])]
    #[case(Boundary::Acronym, "XMLRequest", vec!["XML", "Request"])]
    #[case(Boundary::LowerDigit, "abc123", vec!["abc", "123"])]
    #[case(Boundary::UpperDigit, "ABC123", vec!["ABC", "123"])]
    #[case(Boundary::DigitLower, "123abc", vec!["123", "abc"])]
    #[case(Boundary::DigitUpper, "123ABC", vec!["123", "ABC"])]
    fn split_on_boundary(
        #[case] boundary: Boundary,
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        assert_eq!(split(&input, &[boundary]), expected);
    }

    #[test]
    fn split_on_multiple_delimiters() {
        let s = "aaa-bbb_ccc ddd ddd-eee";
        let v = split(
            &s,
            &[Boundary::Space, Boundary::Underscore, Boundary::Hyphen],
        );
        assert_eq!(v, vec!["aaa", "bbb", "ccc", "ddd", "ddd", "eee"]);
    }

    #[test]
    fn boundaries_found_in_string() {
        // upper lower is no longer a default
        assert_eq!(Boundary::defaults_from(".Aaaa"), Vec::<Boundary>::new());
        assert_eq!(
            Boundary::defaults_from("a8.Aa.aA"),
            vec![Boundary::LowerUpper, Boundary::LowerDigit]
        );
        assert_eq!(
            Boundary::defaults_from("b1B1b"),
            Boundary::digits().to_vec()
        );
        assert_eq!(
            Boundary::defaults_from("AAa -_"),
            vec![
                Boundary::Underscore,
                Boundary::Hyphen,
                Boundary::Space,
                Boundary::Acronym,
            ]
        );
    }
}
