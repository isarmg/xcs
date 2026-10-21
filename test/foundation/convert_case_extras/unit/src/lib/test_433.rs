#[cfg(test)]
mod test {
    use super::*;

    use convert_case::Casing;

    #[test]
    fn toggle_case() {
        assert_eq!("test_toggle".to_case(case::TOGGLE), "tEST tOGGLE");
    }

    #[cfg(feature = "random")]
    #[test]
    fn pseudo_no_triples() {
        let words = vec!["abcdefg", "hijklmnop", "qrstuv", "wxyz"];
        for _ in 0..5 {
            let new = pattern::PSEUDO_RANDOM.mutate(&words).join("");
            let mut iter = new
                .chars()
                .zip(new.chars().skip(1))
                .zip(new.chars().skip(2));
            assert!(!iter
                .clone()
                .any(|((a, b), c)| a.is_lowercase() && b.is_lowercase() && c.is_lowercase()));
            assert!(
                !iter.any(|((a, b), c)| a.is_uppercase() && b.is_uppercase() && c.is_uppercase())
            );
        }
    }

    #[cfg(feature = "random")]
    #[test]
    fn randoms_are_random() {
        let words = vec!["abcdefg", "hijklmnop", "qrstuv", "wxyz"];

        for _ in 0..5 {
            let transformed = pattern::PSEUDO_RANDOM.mutate(&words);
            assert_ne!(words, transformed);
            let transformed = pattern::RANDOM.mutate(&words);
            assert_ne!(words, transformed);
        }
    }
}
