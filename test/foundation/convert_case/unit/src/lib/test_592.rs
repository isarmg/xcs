#[cfg(test)]
mod test {
    use super::*;

    use alloc::vec;

    #[test]
    fn lossless_against_lossless() {
        let examples = vec![
            (Case::Snake, "my_variable_22_name"),
            (Case::Constant, "MY_VARIABLE_22_NAME"),
            (Case::Ada, "My_Variable_22_Name"),
            (Case::Kebab, "my-variable-22-name"),
            (Case::Cobol, "MY-VARIABLE-22-NAME"),
            (Case::Train, "My-Variable-22-Name"),
            (Case::Pascal, "MyVariable22Name"),
            (Case::Camel, "myVariable22Name"),
            (Case::Lower, "my variable 22 name"),
            (Case::Upper, "MY VARIABLE 22 NAME"),
            (Case::Title, "My Variable 22 Name"),
            (Case::Sentence, "My variable 22 name"),
        ];

        for (case_a, str_a) in &examples {
            for (case_b, str_b) in &examples {
                assert_eq!(str_b.to_case(*case_a), *str_a);
                assert_eq!(str_b.from_case(*case_b).to_case(*case_a), *str_a);
            }
        }
    }

    #[test]
    fn obvious_default_parsing() {
        let examples = vec![
            "SuperMario64Game",
            "super-mario64-game",
            "superMario64 game",
            "Super Mario 64_game",
            "SUPERMario 64-game",
            "super_mario-64 game",
        ];

        for example in examples {
            assert_eq!(example.to_case(Case::Snake), "super_mario_64_game");
        }
    }

    #[test]
    fn multiline_strings() {
        assert_eq!("one\ntwo\nthree".to_case(Case::Title), "One\ntwo\nthree");
    }

    #[test]
    fn camel_case_acronyms() {
        assert_eq!(
            "XMLHttpRequest".from_case(Case::Camel).to_case(Case::Snake),
            "xml_http_request"
        );
        assert_eq!(
            "XMLHttpRequest"
                .from_case(Case::UpperCamel)
                .to_case(Case::Snake),
            "xml_http_request"
        );
        assert_eq!(
            "XMLHttpRequest"
                .from_case(Case::Pascal)
                .to_case(Case::Snake),
            "xml_http_request"
        );
    }

    #[test]
    fn leading_tailing_double_delimiters() {
        let words = ["first", "second"];
        let delimited_cases = &[
            Case::Snake,
            Case::Kebab,
            Case::Lower,
            Case::Custom {
                boundaries: &[Boundary::Custom {
                    condition: |s| *s.get(0).unwrap() == ".",
                    start: 0,
                    len: 1,
                }],
                pattern: Pattern::Lowercase,
                delimiter: ".",
            },
        ];

        for &case in delimited_cases {
            let delim = case.delimiter();
            let double = format!("{delim}{delim}");

            let identifiers = [
                format!("{delim}{}", words.join(delim)),
                format!("{}{delim}", words.join(delim)),
                format!("{delim}{}{delim}", words.join(delim)),
                format!("{}", words.join(&double)),
                format!("{delim}{}", words.join(&double)),
                format!("{}{delim}", words.join(&double)),
                format!("{delim}{}{delim}", words.join(&double)),
            ];

            for identifier in identifiers {
                assert_eq!(identifier.to_case(case), identifier);
                assert_eq!(identifier.from_case(case).to_case(case), identifier);
            }
        }
    }

    #[test]
    fn early_word_boundaries() {
        assert_eq!(
            "aBagel".from_case(Case::Camel).to_case(Case::Snake),
            "a_bagel"
        );
    }

    #[test]
    fn late_word_boundaries() {
        assert_eq!(
            "teamA".from_case(Case::Camel).to_case(Case::Snake),
            "team_a"
        );
    }

    #[test]
    fn empty_string() {
        for (case_a, case_b) in Case::all_cases()
            .into_iter()
            .zip(Case::all_cases().into_iter())
        {
            assert_eq!("", "".from_case(*case_a).to_case(*case_b));
        }
    }

    #[test]
    fn default_all_boundaries() {
        assert_eq!(
            "ABC-abc_abcAbc ABCAbc".to_case(Case::Snake),
            "abc_abc_abc_abc_abc_abc"
        );
        assert_eq!("8a8A8".to_case(Case::Snake), "8_a_8_a_8");
    }

    #[test]
    fn remove_boundaries() {
        assert_eq!(
            "M02S05BinaryTrees.pdf"
                .from_case(Case::Pascal)
                .remove_boundaries(&[Boundary::UpperDigit])
                .to_case(Case::Snake),
            "m02_s05_binary_trees.pdf"
        );
    }

    #[test]
    fn with_boundaries() {
        assert_eq!(
            "my_dumbFileName"
                .set_boundaries(&[Boundary::Underscore, Boundary::LowerUpper])
                .to_case(Case::Kebab),
            "my-dumb-file-name"
        );
    }

    // From issue https://github.com/rutrum/convert-case/issues/4
    // From issue https://github.com/rutrum/convert-case/issues/8
    #[test]
    fn unicode_words() {
        let strings = &["ПЕРСПЕКТИВА24", "música moderna"];
        for s in strings {
            for &case in Case::all_cases() {
                assert!(!s.to_case(case).is_empty());
            }
            for &from in Case::all_cases() {
                for &to in Case::all_cases() {
                    assert!(!s.from_case(from).to_case(to).is_empty());
                }
            }
        }
    }

    // idea for asserting the associated boundaries are correct
    #[test]
    fn appropriate_associated_boundaries() {
        let word_groups = &[
            vec!["my", "var", "name"],
            vec!["MY", "var", "Name"],
            vec!["another", "vAR"],
            vec!["XML", "HTTP", "Request"],
        ];

        for words in word_groups {
            for case in Case::all_cases() {
                if case == &Case::Flat || case == &Case::UpperFlat {
                    continue;
                }
                assert_eq!(
                    case.pattern().mutate(&split(
                        &case.pattern().mutate(words).join(case.delimiter()),
                        case.boundaries()
                    )),
                    case.pattern().mutate(words),
                    "Test boundaries on Case::{:?} with {:?}",
                    case,
                    words,
                );
            }
        }
    }
}
