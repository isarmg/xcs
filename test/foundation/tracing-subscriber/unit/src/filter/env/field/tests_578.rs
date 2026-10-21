#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[derive(Debug)]
    #[allow(dead_code)]
    struct MyStruct {
        answer: usize,
        question: &'static str,
    }

    #[test]
    fn debug_struct_match() {
        let my_struct = MyStruct {
            answer: 42,
            question: "life, the universe, and everything",
        };

        let pattern = "MyStruct { answer: 42, question: \"life, the universe, and everything\" }";

        assert_eq!(
            format!("{:?}", my_struct),
            pattern,
            "`MyStruct`'s `Debug` impl doesn't output the expected string"
        );

        let matcher = MatchDebug {
            pattern: pattern.into(),
        };
        assert!(matcher.debug_matches(&my_struct))
    }

    #[test]
    fn debug_struct_not_match() {
        let my_struct = MyStruct {
            answer: 42,
            question: "what shall we have for lunch?",
        };

        let pattern = "MyStruct { answer: 42, question: \"life, the universe, and everything\" }";

        assert_eq!(
            format!("{:?}", my_struct),
            "MyStruct { answer: 42, question: \"what shall we have for lunch?\" }",
            "`MyStruct`'s `Debug` impl doesn't output the expected string"
        );

        let matcher = MatchDebug {
            pattern: pattern.into(),
        };
        assert!(!matcher.debug_matches(&my_struct))
    }
}
