#[cfg(test)]
mod test {
    use crate::Case;
    use crate::Converter;

    use super::*;

    #[test]
    fn mutate_empty_strings() {
        for word_pattern in [lowercase_word, uppercase_word, capital_word] {
            assert_eq!(String::new(), word_pattern(&String::new()))
        }
    }

    #[test]
    fn filtering_with_remove_empty() {
        let conv = Converter::new()
            .from_case(Case::Kebab)
            .set_patterns(&[Pattern::RemoveEmpty, Pattern::Camel]);

        assert_eq!(conv.convert("--leading-delims"), "leadingDelims");
    }

    #[test]
    fn remove_empty_pattern() {
        assert_eq!(
            Pattern::RemoveEmpty.mutate(&["", "first", "", "second", ""]),
            vec!["first", "second"]
        );
        assert_eq!(Pattern::RemoveEmpty.mutate(&["only"]), vec!["only"]);
        assert_eq!(
            Pattern::RemoveEmpty.mutate(&["", "", ""]),
            Vec::<String>::new()
        );
    }
}
