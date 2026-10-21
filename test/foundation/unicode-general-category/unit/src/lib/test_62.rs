#[cfg(test)]
mod test {
    use super::{get_general_category, GeneralCategory};

    #[test]
    fn test_get_category() {
        assert_eq!(get_general_category('a'), GeneralCategory::LowercaseLetter);
        assert_eq!(get_general_category('.'), GeneralCategory::OtherPunctuation);
        assert_eq!(get_general_category('カ'), GeneralCategory::OtherLetter);
        assert_eq!(get_general_category('🦳'), GeneralCategory::OtherSymbol);
    }

    #[test]
    fn test_abbreviation() {
        let cat = get_general_category('a');
        assert_eq!(cat.abbreviation(), "Ll");
    }
}
