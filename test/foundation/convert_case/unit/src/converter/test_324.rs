#[cfg(test)]
mod test {
    use super::*;
    use crate::Casing;

    #[test]
    fn snake_converter_from_case() {
        let conv = Converter::new().to_case(Case::Snake);
        let s = String::from("my var name");
        assert_eq!(s.to_case(Case::Snake), conv.convert(s));
    }

    #[test]
    fn snake_converter_from_scratch() {
        let conv = Converter::new()
            .set_delimiter("_")
            .set_patterns(&[Pattern::Lowercase]);
        let s = String::from("my var name");
        assert_eq!(s.to_case(Case::Snake), conv.convert(s));
    }

    #[test]
    fn custom_pattern() {
        let conv = Converter::new()
            .to_case(Case::Snake)
            .set_patterns(&[Pattern::Sentence]);
        assert_eq!(conv.convert("bjarne case"), "Bjarne_case");
    }

    #[test]
    fn custom_delim() {
        let conv = Converter::new().set_delimiter("..");
        assert_eq!(conv.convert("ohMy"), "oh..My");
    }

    #[test]
    fn no_delim() {
        let conv = Converter::new()
            .from_case(Case::Title)
            .to_case(Case::Kebab)
            .set_delimiter("");
        assert_eq!(conv.convert("Just Flat"), "justflat");
    }

    #[test]
    fn no_digit_boundaries() {
        let conv = Converter::new()
            .remove_boundaries(&Boundary::digits())
            .to_case(Case::Snake);
        assert_eq!(conv.convert("Test 08Bound"), "test_08bound");
        assert_eq!(conv.convert("a8aA8A"), "a8a_a8a");
    }

    #[test]
    fn remove_boundary() {
        let conv = Converter::new()
            .remove_boundary(Boundary::DigitUpper)
            .to_case(Case::Snake);
        assert_eq!(conv.convert("Test 08Bound"), "test_08bound");
        assert_eq!(conv.convert("a8aA8A"), "a_8_a_a_8a");
    }

    #[test]
    fn add_boundary() {
        let conv = Converter::new()
            .from_case(Case::Snake)
            .to_case(Case::Kebab)
            .add_boundary(Boundary::LowerUpper);
        assert_eq!(conv.convert("word_wordWord"), "word-word-word");
    }

    #[test]
    fn add_boundaries() {
        let conv = Converter::new()
            .from_case(Case::Snake)
            .to_case(Case::Kebab)
            .add_boundaries(&[Boundary::LowerUpper, Boundary::UpperLower]);
        assert_eq!(conv.convert("word_wordWord"), "word-word-w-ord");
    }

    #[test]
    fn twice() {
        let s = "myVarName".to_string();
        let conv = Converter::new().to_case(Case::Snake);
        let snake = conv.convert(&s);
        let kebab = s.to_case(Case::Kebab);
        assert_eq!(snake.to_case(Case::Camel), kebab.to_case(Case::Camel));
    }

    #[test]
    fn reuse_after_change() {
        let conv = Converter::new().from_case(Case::Snake).to_case(Case::Kebab);
        assert_eq!(conv.convert("word_wordWord"), "word-wordword");

        let conv = conv.add_boundary(Boundary::LowerUpper);
        assert_eq!(conv.convert("word_wordWord"), "word-word-word");
    }

    #[test]
    fn explicit_boundaries() {
        let conv = Converter::new()
            .set_boundaries(&[
                Boundary::DigitLower,
                Boundary::DigitUpper,
                Boundary::Acronym,
            ])
            .to_case(Case::Snake);
        assert_eq!(
            conv.convert("section8lesson2HTTPRequests"),
            "section8_lesson2_http_requests"
        );
    }
}
