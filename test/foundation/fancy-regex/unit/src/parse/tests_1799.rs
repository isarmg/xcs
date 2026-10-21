#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use alloc::string::{String, ToString};
    use alloc::{format, vec};

    use crate::parse::{
        make_group, make_literal, make_literal_case_insensitive, parse_id, parse_unrestricted_name,
        remap_unicode_property_if_necessary,
    };
    use crate::{Absent, Assertion, BacktrackingControlVerb, Expr};
    use crate::{LookAround::*, RegexOptions, SyntaxConfig};

    fn p(s: &str) -> Expr {
        Expr::parse_tree(s).unwrap().expr
    }

    fn parse_oniguruma(s: &str) -> crate::Result<Expr> {
        let mut options = RegexOptions::default();
        options.oniguruma_mode = true;
        options.syntaxc = options.syntaxc.multi_line(true);
        Expr::parse_tree_with_flags(s, options.compute_flags()).map(|tree| tree.expr)
    }

    #[cfg_attr(feature = "track_caller", track_caller)]
    fn fail(s: &str) {
        assert!(
            Expr::parse_tree(s).is_err(),
            "Expected parse error, but was: {:?}",
            Expr::parse_tree(s)
        );
    }

    #[cfg_attr(feature = "track_caller", track_caller)]
    fn assert_error(re: &str, expected_error: &str) {
        let result = Expr::parse_tree(re);
        assert!(
            result.is_err(),
            "Expected parse error, but was: {:?}",
            result
        );
        assert_eq!(&format!("{}", result.err().unwrap()), expected_error);
    }

    #[cfg_attr(feature = "track_caller", track_caller)]
    fn assert_error_oniguruma(re: &str, expected_error: &str) {
        let result = parse_oniguruma(re);
        assert!(
            result.is_err(),
            "Expected parse error in Oniguruma mode, but was: {:?}",
            result
        );
        assert_eq!(&format!("{}", result.err().unwrap()), expected_error);
    }

    #[test]
    fn empty() {
        assert_eq!(p(""), Expr::Empty);
    }

    #[test]
    fn any() {
        assert_eq!(
            p("."),
            Expr::Any {
                newline: false,
                crlf: false
            }
        );
        assert_eq!(
            p("(?s:.)"),
            Expr::Any {
                newline: true,
                crlf: false
            }
        );
    }

    #[test]
    fn start_text() {
        assert_eq!(p("^"), Expr::Assertion(Assertion::StartText));
    }

    #[test]
    fn end_text() {
        assert_eq!(p("$"), Expr::Assertion(Assertion::EndText));
    }

    #[test]
    fn end_text_before_empty_lines() {
        assert_eq!(
            p("\\Z"),
            Expr::Assertion(Assertion::EndTextIgnoreTrailingNewlines { crlf: false })
        );
    }

    #[test]
    fn literal() {
        assert_eq!(p("a"), make_literal("a"));
    }

    #[test]
    fn literal_special() {
        assert_eq!(p("}"), make_literal("}"));
        assert_eq!(p("]"), make_literal("]"));
    }

    #[test]
    fn parse_id_test() {
        use crate::parse::ParsedId;
        fn create_id(id: &str, relative: Option<isize>, skip: usize) -> Option<ParsedId<'_>> {
            Some(ParsedId { id, relative, skip })
        }
        assert_eq!(parse_id("foo.", "", "", true), create_id("foo", None, 3));
        assert_eq!(parse_id("1.", "", "", true), create_id("1", None, 1));
        assert_eq!(parse_id("{foo}", "{", "}", true), create_id("foo", None, 5));
        assert_eq!(parse_id("{foo.", "{", "}", true), None);
        assert_eq!(parse_id("{foo", "{", "}", true), None);
        assert_eq!(parse_id("{}", "{", "}", true), None);
        assert_eq!(parse_id("", "", "", true), None);
        assert_eq!(parse_id("{-1}", "{", "}", true), create_id("", Some(-1), 4));
        assert_eq!(parse_id("{-1}", "{", "}", false), None);
        assert_eq!(parse_id("{-a}", "{", "}", true), None);
        assert_eq!(parse_id("{-a}", "{", "}", false), None);
        assert_eq!(parse_id("{+a}", "{", "}", false), None);
        assert_eq!(parse_id("+a", "", "", false), None);
        assert_eq!(parse_id("-a", "", "", false), None);
        assert_eq!(parse_id("2+a", "", "", false), create_id("2", None, 1));
        assert_eq!(parse_id("2-a", "", "", false), create_id("2", None, 1));

        assert_eq!(parse_id("<+1>", "<", ">", true), create_id("", Some(1), 4));
        assert_eq!(parse_id("<-3>", "<", ">", true), create_id("", Some(-3), 4));
        assert_eq!(
            parse_id("<n+1>", "<", ">", true),
            create_id("n", Some(1), 5)
        );
        assert_eq!(
            parse_id("<n-1>", "<", ">", true),
            create_id("n", Some(-1), 5)
        );
        assert_eq!(parse_id("<>", "<", ">", true), None);
        assert_eq!(parse_id("<", "<", ">", true), None);
        assert_eq!(parse_id("<+0>", "<", ">", true), None);
        assert_eq!(parse_id("<-0>", "<", ">", true), None);
        assert_eq!(
            parse_id("<n+0>", "<", ">", true),
            create_id("n", Some(0), 5)
        );
        assert_eq!(
            parse_id("<n-0>", "<", ">", true),
            create_id("n", Some(0), 5)
        );
        assert_eq!(
            parse_id("<2-0>", "<", ">", true),
            create_id("2", Some(0), 5)
        );
        assert_eq!(
            parse_id("<2+0>", "<", ">", true),
            create_id("2", Some(0), 5)
        );
        assert_eq!(
            parse_id("<2+1>", "<", ">", true),
            create_id("2", Some(1), 5)
        );
        assert_eq!(
            parse_id("<2-1>", "<", ">", true),
            create_id("2", Some(-1), 5)
        );
    }

    #[test]
    fn parse_unrestricted_name_test() {
        assert_eq!(parse_unrestricted_name("<foo>", "<", ">"), Some(("foo", 5)));
        assert_eq!(
            parse_unrestricted_name("<foo-bar>", "<", ">"),
            Some(("foo-bar", 9))
        );
        assert_eq!(
            parse_unrestricted_name("<hello world>", "<", ">"),
            Some(("hello world", 13))
        );
        assert_eq!(parse_unrestricted_name("< >", "<", ">"), Some((" ", 3)));
        assert_eq!(parse_unrestricted_name("<#!@>", "<", ">"), Some(("#!@", 5)));
        assert_eq!(
            parse_unrestricted_name("'foo-bar'", "'", "'"),
            Some(("foo-bar", 9))
        );
        // Empty name rejected
        assert_eq!(parse_unrestricted_name("<>", "<", ">"), None);
        assert_eq!(parse_unrestricted_name("''", "'", "'"), None);
        // Missing close delimiter
        assert_eq!(parse_unrestricted_name("<foo", "<", ">"), None);
        // Too short
        assert_eq!(parse_unrestricted_name("<", "<", ">"), None);
        // Doesn't start with open
        assert_eq!(parse_unrestricted_name("foo>", "<", ">"), None);
        // No open/close delimiters (empty strings)
        assert_eq!(parse_unrestricted_name("foo)", "", ")"), Some(("foo", 4)));
        assert_eq!(parse_unrestricted_name(")", "", ")"), None);
    }

    #[test]
    fn literal_unescaped_opening_curly() {
        // `{` in position where quantifier is not allowed results in literal `{`
        assert_eq!(p("{"), make_literal("{"));
        assert_eq!(p("({)"), make_group(make_literal("{"),));
        assert_eq!(
            p("a|{"),
            Expr::Alt(vec![make_literal("a"), make_literal("{"),])
        );
        assert_eq!(
            p("{{2}"),
            Expr::Repeat {
                child: Box::new(make_literal("{")),
                lo: 2,
                hi: 2,
                greedy: true
            }
        );
    }

    #[test]
    fn literal_escape() {
        assert_eq!(p("\\'"), make_literal("'"));
        assert_eq!(p("\\\""), make_literal("\""));
        assert_eq!(p("\\ "), make_literal(" "));
        assert_eq!(p("\\xA0"), make_literal("\u{A0}"));
        assert_eq!(p("\\x{1F4A9}"), make_literal("\u{1F4A9}"));
        assert_eq!(p("\\x{000000B7}"), make_literal("\u{B7}"));
        assert_eq!(p("\\u21D2"), make_literal("\u{21D2}"));
        assert_eq!(p("\\u{21D2}"), make_literal("\u{21D2}"));
        assert_eq!(p("\\u21D2x"), p("\u{21D2}x"));
        assert_eq!(p("\\U0001F60A"), make_literal("\u{1F60A}"));
        assert_eq!(p("\\U{0001F60A}"), make_literal("\u{1F60A}"));
    }

    #[test]
    fn hex_escape() {
        assert_eq!(
            p("\\h"),
            Expr::Delegate {
                inner: String::from("[0-9A-Fa-f]"),
                casei: false,
            }
        );
        assert_eq!(
            p("\\H"),
            Expr::Delegate {
                inner: String::from("[^0-9A-Fa-f]"),
                casei: false,
            }
        );
    }

    #[test]
    fn invalid_escape() {
        assert_error(
            "\\",
            "Parsing error at position 0: Backslash without following character",
        );
        assert_error("\\q", "Parsing error at position 0: Invalid escape: \\q");
        assert_error("\\u", "Parsing error at position 2: Invalid hex escape");
        assert_error("\\U", "Parsing error at position 2: Invalid hex escape");
        assert_error("\\x", "Parsing error at position 2: Invalid hex escape");
        assert_error("\\xAG", "Parsing error at position 2: Invalid hex escape");
        assert_error("\\xA", "Parsing error at position 2: Invalid hex escape");
        assert_error("\\x{}", "Parsing error at position 2: Invalid hex escape");
        assert_error("\\x{AG}", "Parsing error at position 2: Invalid hex escape");
        assert_error("\\x{42", "Parsing error at position 2: Invalid hex escape");
        assert_error(
            "\\x{D800}",
            "Parsing error at position 2: Invalid codepoint for hex or unicode escape",
        );
        assert_error(
            "\\x{110000}",
            "Parsing error at position 2: Invalid codepoint for hex or unicode escape",
        );
        assert_error("\\u123", "Parsing error at position 2: Invalid hex escape");
        assert_error("\\u123x", "Parsing error at position 2: Invalid hex escape");
        assert_error("\\u{}", "Parsing error at position 2: Invalid hex escape");
        assert_error(
            "\\U1234567",
            "Parsing error at position 2: Invalid hex escape",
        );
        assert_error("\\U{}", "Parsing error at position 2: Invalid hex escape");
    }

    #[test]
    fn concat() {
        assert_eq!(
            p("ab"),
            Expr::Concat(vec![make_literal("a"), make_literal("b"),])
        );
    }

    #[test]
    fn alt() {
        assert_eq!(
            p("a|b"),
            Expr::Alt(vec![make_literal("a"), make_literal("b"),])
        );
    }

    #[test]
    fn group() {
        assert_eq!(p("(a)"), make_group(make_literal("a"),));
    }

    #[test]
    fn named_group() {
        assert_eq!(p("(?'name'a)"), make_group(make_literal("a"),));
        assert_eq!(p("(?<name>a)"), make_group(make_literal("a"),));
    }

    #[test]
    fn unrestricted_named_group() {
        // Hyphens in group names
        assert_eq!(p("(?<foo-bar>a)"), make_group(make_literal("a")));
        assert_eq!(p("(?'foo-bar'a)"), make_group(make_literal("a")));
        assert_eq!(p("(?P<foo-bar>a)"), make_group(make_literal("a")));

        // Multiple hyphens / CSS-like names
        assert_eq!(p("(?<data-value-1>a)"), make_group(make_literal("a")));

        // Leading/trailing hyphens
        assert_eq!(p("(?<-foo>a)"), make_group(make_literal("a")));
        assert_eq!(p("(?<foo->a)"), make_group(make_literal("a")));

        // Special characters in group names
        assert_eq!(p("(?<#>a)"), make_group(make_literal("a")));
        assert_eq!(p("(?<hello, world!>a)"), make_group(make_literal("a")));
        assert_eq!(p("(?<a b c>a)"), make_group(make_literal("a")));

        // Emoji in group names
        assert_eq!(p("(?<\u{1F3AF}>a)"), make_group(make_literal("a")));

        // Delimiter chars that don't conflict (single-quote inside angle brackets, etc.)
        assert_eq!(p("(?<a'b>a)"), make_group(make_literal("a")));
        assert_eq!(p("(?'a>b'a)"), make_group(make_literal("a")));

        // Name that looks numeric with hyphens (treated as named, not numeric)
        let tree = Expr::parse_tree("(?<1-2>a)").unwrap();
        assert_eq!(tree.named_groups.get("1-2"), Some(&1));

        // Verify named_groups map is populated correctly for hyphenated names
        let tree = Expr::parse_tree("(?<foo-bar>a)").unwrap();
        assert_eq!(tree.named_groups.get("foo-bar"), Some(&1));

        let tree = Expr::parse_tree("(?P<data-value>a)").unwrap();
        assert_eq!(tree.named_groups.get("data-value"), Some(&1));
    }

    #[test]
    fn unrestricted_named_group_empty_still_rejected() {
        // Empty names are still rejected
        fail("(?<>a)");
        fail("(?P<>a)");
        fail("(?''a)");
    }

    #[test]
    fn unrestricted_named_group_missing_close_still_rejected() {
        // Missing close delimiter
        fail("(?<foo-bar");
        fail("(?'foo-bar");
        fail("(?P<foo-bar");
    }

    #[test]
    fn group_repeat() {
        assert_eq!(
            p("(a){2}"),
            Expr::Repeat {
                child: Box::new(make_group(make_literal("a"))),
                lo: 2,
                hi: 2,
                greedy: true
            }
        );
    }

    #[test]
    fn repeat() {
        assert_eq!(
            p("a{2,42}"),
            Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 2,
                hi: 42,
                greedy: true
            }
        );
        assert_eq!(
            p("a{2,}"),
            Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 2,
                hi: usize::MAX,
                greedy: true
            }
        );
        assert_eq!(
            p("a{2}"),
            Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 2,
                hi: 2,
                greedy: true
            }
        );
        assert_eq!(
            p("a{,2}"),
            Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 0,
                hi: 2,
                greedy: true
            }
        );

        assert_eq!(
            p("a{2,42}?"),
            Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 2,
                hi: 42,
                greedy: false
            }
        );
        assert_eq!(
            p("a{2,}?"),
            Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 2,
                hi: usize::MAX,
                greedy: false
            }
        );
        assert_eq!(
            p("a{2}?"),
            Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 2,
                hi: 2,
                greedy: false
            }
        );
        assert_eq!(
            p("a{,2}?"),
            Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 0,
                hi: 2,
                greedy: false
            }
        );
    }

    #[test]
    fn invalid_repeat() {
        // Invalid repeat syntax results in literal
        assert_eq!(
            p("a{"),
            Expr::Concat(vec![make_literal("a"), make_literal("{"),])
        );
        assert_eq!(
            p("a{6"),
            Expr::Concat(vec![
                make_literal("a"),
                make_literal("{"),
                make_literal("6"),
            ])
        );
        assert_eq!(
            p("a{6,"),
            Expr::Concat(vec![
                make_literal("a"),
                make_literal("{"),
                make_literal("6"),
                make_literal(","),
            ])
        );
        assert_eq!(
            p("a{1,A}"),
            Expr::Concat(vec![
                make_literal("a"),
                make_literal("{"),
                make_literal("1"),
                make_literal(","),
                make_literal("A"),
                make_literal("}"),
            ])
        );
        assert_eq!(
            p("a{1,2A}"),
            Expr::Concat(vec![
                make_literal("a"),
                make_literal("{"),
                make_literal("1"),
                make_literal(","),
                make_literal("2"),
                make_literal("A"),
                make_literal("}"),
            ])
        );
    }

    #[test]
    fn quantifiers_on_assertions_in_regex_mode() {
        assert_eq!(
            p(r"\b{1}"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::WordBoundary)),
                lo: 1,
                hi: 1,
                greedy: true
            }
        );
        assert_eq!(
            p(r"\B{2}"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::NotWordBoundary)),
                lo: 2,
                hi: 2,
                greedy: true
            }
        );
        assert_eq!(
            p(r"^{3}"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::StartText)),
                lo: 3,
                hi: 3,
                greedy: true
            }
        );
        assert_eq!(
            p(r"${1,5}"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::EndText)),
                lo: 1,
                hi: 5,
                greedy: true
            }
        );
        assert_eq!(
            p(r"\A*"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::StartText)),
                lo: 0,
                hi: usize::MAX,
                greedy: true
            }
        );
        assert_eq!(
            p(r"\z+"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::EndText)),
                lo: 1,
                hi: usize::MAX,
                greedy: true
            }
        );
        assert_eq!(
            p(r"^?"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::StartText)),
                lo: 0,
                hi: 1,
                greedy: true
            }
        );
        assert_eq!(
            p(r"${2}"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::EndText)),
                lo: 2,
                hi: 2,
                greedy: true
            }
        );
        assert_eq!(
            p(r"(?m)^?"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::StartLine { crlf: false })),
                lo: 0,
                hi: 1,
                greedy: true
            }
        );
        assert_eq!(
            p(r"(?m)${2}"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::EndLine { crlf: false })),
                lo: 2,
                hi: 2,
                greedy: true
            }
        );
        assert_eq!(
            p(r"\b+"),
            Expr::Repeat {
                child: Box::new(Expr::Assertion(Assertion::WordBoundary)),
                lo: 1,
                hi: usize::MAX,
                greedy: true
            }
        );
    }

    #[test]
    fn delegate_zero() {
        assert_eq!(p("\\b"), Expr::Assertion(Assertion::WordBoundary),);
        assert_eq!(p("\\B"), Expr::Assertion(Assertion::NotWordBoundary),);
    }

    #[test]
    fn delegate_named_group() {
        assert_eq!(
            p("\\p{Greek}"),
            Expr::Delegate {
                inner: String::from("\\p{greek}"),
                casei: false,
            }
        );
        assert_eq!(
            p("\\pL"),
            Expr::Delegate {
                inner: String::from("\\pL"),
                casei: false,
            }
        );
        assert_eq!(
            p("\\P{Greek}"),
            Expr::Delegate {
                inner: String::from("\\P{greek}"),
                casei: false,
            }
        );
        assert_eq!(
            p("\\PL"),
            Expr::Delegate {
                inner: String::from("\\PL"),
                casei: false,
            }
        );
        assert_eq!(
            p("(?i)\\p{Ll}"),
            Expr::Delegate {
                inner: String::from("\\p{ll}"),
                casei: true,
            }
        );
    }

    #[test]
    fn backref() {
        let tree = Expr::parse_tree(r"(.)\1").unwrap();
        assert_eq!(
            tree.expr,
            Expr::Concat(vec![
                make_group(Expr::Any {
                    newline: false,
                    crlf: false
                }),
                Expr::Backref {
                    group: 1,
                    casei: false,
                },
            ])
        );
        assert!(tree.numeric_capture_group_references);
    }

    #[test]
    fn named_backref() {
        let tree = Expr::parse_tree(r"(?<i>.)\k<i>").unwrap();
        assert_eq!(
            tree.expr,
            Expr::Concat(vec![
                make_group(Expr::Any {
                    newline: false,
                    crlf: false
                }),
                Expr::Backref {
                    group: 1,
                    casei: false,
                },
            ])
        );
        assert!(!tree.numeric_capture_group_references);
    }

    #[test]
    fn relative_backref() {
        let tree = Expr::parse_tree(r"(a)(.)\k<-1>").unwrap();
        assert_eq!(
            tree.expr,
            Expr::Concat(vec![
                make_group(make_literal("a")),
                make_group(Expr::Any {
                    newline: false,
                    crlf: false
                }),
                Expr::Backref {
                    group: 2,
                    casei: false,
                },
            ])
        );
        // this doesn't count as a numeric reference
        assert!(!tree.numeric_capture_group_references);

        let tree = Expr::parse_tree(r"(a)\k<+1>(.)").unwrap();
        assert_eq!(
            tree.expr,
            Expr::Concat(vec![
                make_group(make_literal("a")),
                Expr::Backref {
                    group: 2,
                    casei: false,
                },
                make_group(Expr::Any {
                    newline: false,
                    crlf: false
                }),
            ])
        );
        // this doesn't count as a numeric reference
        assert!(!tree.numeric_capture_group_references);

        // "-" is now a valid unrestricted group name
        p("(?P<->.)");
        fail("(.)(?P=-)");
        fail(r"(a)\k<-0>(.)");
        fail(r"(a)\k<+0>(.)");
        fail(r"(a)\k<+>(.)");
        fail(r"(a)\k<->(.)");
        fail(r"(a)\k<>(.)");
    }

    #[test]
    fn relative_backref_with_recursion_level() {
        assert_eq!(
            p(r"()\k<1+3>"),
            Expr::Concat(vec![
                make_group(Expr::Empty),
                Expr::BackrefWithRelativeRecursionLevel {
                    group: 1,
                    relative_level: 3,
                    casei: false,
                },
            ]),
        );

        assert_eq!(
            p(r"()\k<1-0>"),
            Expr::Concat(vec![
                make_group(Expr::Empty),
                Expr::BackrefWithRelativeRecursionLevel {
                    group: 1,
                    relative_level: 0,
                    casei: false,
                },
            ]),
        );

        assert_eq!(
            p(r"(?<n>)\k<n+3>"),
            Expr::Concat(vec![
                make_group(Expr::Empty),
                Expr::BackrefWithRelativeRecursionLevel {
                    group: 1,
                    relative_level: 3,
                    casei: false,
                },
            ]),
        );

        assert_eq!(
            p(r"(?<n>)\k<n-3>"),
            Expr::Concat(vec![
                make_group(Expr::Empty),
                Expr::BackrefWithRelativeRecursionLevel {
                    group: 1,
                    relative_level: -3,
                    casei: false,
                }
            ]),
        );

        assert_eq!(
            p(r"\A(?<a>|.|(?:(?<b>.)\g<a>\k<b+0>))\z"),
            Expr::Concat(vec![
                Expr::Assertion(Assertion::StartText),
                make_group(Expr::Alt(vec![
                    Expr::Empty,
                    Expr::Any {
                        newline: false,
                        crlf: false
                    },
                    Expr::Concat(vec![
                        make_group(Expr::Any {
                            newline: false,
                            crlf: false
                        }),
                        Expr::SubroutineCall(1),
                        Expr::BackrefWithRelativeRecursionLevel {
                            group: 2,
                            relative_level: 0,
                            casei: false,
                        },
                    ])
                ])),
                Expr::Assertion(Assertion::EndText)
            ]),
        );
    }

    #[test]
    fn relative_subroutine_call() {
        assert_eq!(
            p(r"(a)(.)\g<-1>"),
            Expr::Concat(vec![
                make_group(make_literal("a")),
                make_group(Expr::Any {
                    newline: false,
                    crlf: false
                }),
                Expr::SubroutineCall(2),
            ])
        );

        assert_eq!(
            p(r"(a)\g<+1>(.)"),
            Expr::Concat(vec![
                make_group(make_literal("a")),
                Expr::SubroutineCall(2),
                make_group(Expr::Any {
                    newline: false,
                    crlf: false
                }),
            ])
        );

        fail(r"(a)\g<-0>(.)");
        fail(r"(a)\g<+0>(.)");
        fail(r"(a)\g<+>(.)");
        fail(r"(a)\g<->(.)");
        fail(r"(a)\g<>(.)");
    }

    #[test]
    fn lookaround() {
        assert_eq!(
            p("(?=a)"),
            Expr::LookAround(Box::new(make_literal("a")), LookAhead)
        );
        assert_eq!(
            p("(?!a)"),
            Expr::LookAround(Box::new(make_literal("a")), LookAheadNeg)
        );
        assert_eq!(
            p("(?<=a)"),
            Expr::LookAround(Box::new(make_literal("a")), LookBehind)
        );
        assert_eq!(
            p("(?<!a)"),
            Expr::LookAround(Box::new(make_literal("a")), LookBehindNeg)
        );
    }

    #[test]
    fn shy_group() {
        assert_eq!(
            p("(?:ab)c"),
            Expr::Concat(vec![
                Expr::Concat(vec![make_literal("a"), make_literal("b"),]),
                make_literal("c"),
            ])
        );
    }

    #[test]
    fn flag_state() {
        assert_eq!(
            p("(?s)."),
            Expr::Any {
                newline: true,
                crlf: false
            }
        );
        assert_eq!(
            p("(?s:(?-s:.))"),
            Expr::Any {
                newline: false,
                crlf: false
            }
        );
        assert_eq!(
            p("(?s:.)."),
            Expr::Concat(vec![
                Expr::Any {
                    newline: true,
                    crlf: false
                },
                Expr::Any {
                    newline: false,
                    crlf: false
                },
            ])
        );
        assert_eq!(
            p("(?:(?s).)."),
            Expr::Concat(vec![
                Expr::Any {
                    newline: true,
                    crlf: false
                },
                Expr::Any {
                    newline: false,
                    crlf: false
                },
            ])
        );
    }

    #[test]
    fn flag_multiline() {
        assert_eq!(p("^"), Expr::Assertion(Assertion::StartText));
        assert_eq!(
            p("(?m:^)"),
            Expr::Assertion(Assertion::StartLine { crlf: false })
        );
        assert_eq!(p("$"), Expr::Assertion(Assertion::EndText));
        assert_eq!(
            p("(?m:$)"),
            Expr::Assertion(Assertion::EndLine { crlf: false })
        );
    }

    #[test]
    fn flag_crlf() {
        assert_eq!(
            p("(?mR:^)"),
            Expr::Assertion(Assertion::StartLine { crlf: true })
        );
        assert_eq!(
            p("(?Rm:^)"),
            Expr::Assertion(Assertion::StartLine { crlf: true })
        );
        assert_eq!(
            p("(?mR:$)"),
            Expr::Assertion(Assertion::EndLine { crlf: true })
        );
        // Negating R reverts to LF-only
        assert_eq!(
            p("(?mR)(?-R:^)"),
            Expr::Assertion(Assertion::StartLine { crlf: false })
        );
    }

    #[test]
    fn flag_swap_greed() {
        assert_eq!(p("a*"), p("(?U:a*?)"));
        assert_eq!(p("a*?"), p("(?U:a*)"));
    }

    #[test]
    fn invalid_flags() {
        assert!(Expr::parse_tree("(?").is_err());
        assert!(Expr::parse_tree("(?)").is_err());
        assert!(Expr::parse_tree("(?-)").is_err());
        assert!(Expr::parse_tree("(?-:a)").is_err());
        assert!(Expr::parse_tree("(?q:a)").is_err());
    }

    #[test]
    fn lifetime() {
        assert_eq!(
            p("\\'[a-zA-Z_][a-zA-Z0-9_]*(?!\\')\\b"),
            Expr::Concat(vec![
                make_literal("'"),
                Expr::Delegate {
                    inner: String::from("[a-zA-Z_]"),
                    casei: false,
                },
                Expr::Repeat {
                    child: Box::new(Expr::Delegate {
                        inner: String::from("[a-zA-Z0-9_]"),
                        casei: false,
                    }),
                    lo: 0,
                    hi: usize::MAX,
                    greedy: true
                },
                Expr::LookAround(Box::new(make_literal("'")), LookAheadNeg),
                Expr::Assertion(Assertion::WordBoundary),
            ])
        );
    }

    #[test]
    fn ignore_whitespace() {
        assert_eq!(p("(?x: )"), p(""));
        assert_eq!(p("(?x) | "), p("|"));
        assert_eq!(p("(?x: a )"), p("a"));
        assert_eq!(p("(?x: a # ) bobby tables\n b )"), p("ab"));
        assert_eq!(p("(?x: a | b )"), p("a|b"));
        assert_eq!(p("(?x: ( a b ) )"), p("(ab)"));
        assert_eq!(p("(?x: a + )"), p("a+"));
        assert_eq!(p("(?x: a {2} )"), p("a{2}"));
        assert_eq!(p("(?x: a { 2 } )"), p("a{2}"));
        assert_eq!(p("(?x: a { 2 , } )"), p("a{2,}"));
        assert_eq!(p("(?x: a { , 2 } )"), p("a{,2}"));
        assert_eq!(p("(?x: a { 2 , 3 } )"), p("a{2,3}"));
        assert_eq!(p("(?x: a { 2 , 3 } ? )"), p("a{2,3}?"));
        assert_eq!(p("(?x: ( ? i : . ) )"), p("(?i:.)"));
        assert_eq!(p("(?x: ( ?= a ) )"), p("(?=a)"));
        assert_eq!(p("(?x: [ ] )"), p("[ ]"));
        assert_eq!(p("(?x: [ ^] )"), p("[ ^]"));
        assert_eq!(p("(?x: [a - z] )"), p("[a - z]"));
        assert_eq!(p("(?x: [ \\] \\\\] )"), p("[ \\] \\\\]"));
        assert_eq!(p("(?x: a\\ b )"), p("a b"));
        assert_eq!(p("(?x: a (?-x:#) b )"), p("a#b"));
        assert_eq!(p("(?x: a (?# b))c"), p("ac"));
        assert_eq!(p("(?x: \\b { s t a r t} a )"), p("\\b{start}a"));
        assert_eq!(p("(?x: \\b {end} )"), p("\\b{end}"));
        assert_eq!(p("(?x: \\b { start-half } )"), p("\\b{start-half}"));
        assert_eq!(p("(?x: \\b { e n d - h a l f } )"), p("\\b{end-half}"));
        assert_eq!(p("(?x: \\b{s\nt\ra\trt} a)"), p("\\b{start}a"));
        assert_eq!(p("(?x: \\u { 0 0 3 2} a)"), p("\\u{0032}a"));
        assert_eq!(p("(?x: \\U { 0001 F600 } a)"), p("\\U{0001F600}a"));
        assert_eq!(p("(?x: \\x { 3 2} a)"), p("\\x{32}a"));
        assert_eq!(p("(?x: \\b { s t a # x\nr t} a )"), p("\\b{start}a"));
        assert_eq!(p("(?x: \\b { s t a (?# x)r t} a )"), p("\\b{start}a"));
        assert_eq!(p("(?x: \\u { 0 0 # x\n3 2} a)"), p("\\u{0032}a"));
        assert_eq!(p("(?x: \\u { 0 0 (?# x)3 2} a)"), p("\\u{0032}a"));
    }

    #[test]
    fn comments() {
        assert_eq!(p(r"ab(?# comment)"), p("ab"));
        assert_eq!(p(r"ab(?#)"), p("ab"));
        assert_eq!(p(r"(?# comment 1)(?# comment 2)ab"), p("ab"));
        assert_eq!(p(r"ab(?# comment \))c"), p("abc"));
        assert_eq!(p(r"ab(?# comment \\)c"), p("abc"));
        assert_eq!(p(r"ab(?# comment ()c"), p("abc"));
        assert_eq!(p(r"ab(?# comment)*"), p("ab*"));
        fail(r"ab(?# comment");
        fail(r"ab(?# comment\");
    }

    #[test]
    fn atomic_group() {
        assert_eq!(p("(?>a)"), Expr::AtomicGroup(Box::new(make_literal("a"))));
    }

    #[test]
    fn possessive() {
        assert_eq!(
            p("a++"),
            Expr::AtomicGroup(Box::new(Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 1,
                hi: usize::MAX,
                greedy: true
            }))
        );
        assert_eq!(
            p("a*+"),
            Expr::AtomicGroup(Box::new(Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 0,
                hi: usize::MAX,
                greedy: true
            }))
        );
        assert_eq!(
            p("a?+"),
            Expr::AtomicGroup(Box::new(Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo: 0,
                hi: 1,
                greedy: true
            }))
        );
    }

    #[test]
    fn invalid_backref() {
        // Only syntactic tests (non-decimal or number overflow); see tests in the analyze module
        // for out-of-range group number validation.
        assert_error(
            r".\18446744073709551616",
            "Parsing error at position 2: Invalid back reference",
        ); // overflows usize - not a valid integer
        assert_error(r".\c", "Parsing error at position 1: Invalid escape: \\c");
        // not decimal
    }

    #[test]
    fn invalid_group_name_backref() {
        assert_error(
            "\\k<id>(?<id>.)",
            "Parsing error at position 2: Invalid group name in back reference: id",
        );
    }

    #[test]
    fn invalid_group_name() {
        assert_error(
            "(?<id)",
            "Parsing error at position 1: Could not parse group name",
        );
        assert_error(
            "(?<>)",
            "Parsing error at position 1: Could not parse group name",
        );
        assert_error(
            "\\kxxx<id>",
            "Parsing error at position 2: Could not parse group name",
        );
        // "-" can only be used after a name for relative recursion level, so must be followed by a number
        assert_error(
            "\\k<id-withdash>",
            "Parsing error at position 2: Could not parse group name",
        );
        assert_error(
            "\\k<-id>",
            "Parsing error at position 2: Could not parse group name",
        );
    }

    #[test]
    fn unknown_flag() {
        assert_error(
            "(?-:a)",
            "Parsing error at position 2: Unknown group flag: (?-:",
        );
        assert_error(
            "(?)",
            "Parsing error at position 2: Unknown group flag: (?)",
        );
        assert_error(
            "(?--)",
            "Parsing error at position 2: Unknown group flag: (?--",
        );
        // Check that we don't split on char boundary
        assert_error(
            "(?\u{1F60A})",
            "Parsing error at position 2: Unknown group flag: (?\u{1F60A}",
        );
    }

    #[test]
    fn no_quantifiers_on_lookarounds() {
        assert_error(
            "(?=hello)+",
            "Parsing error at position 9: Target of repeat operator is invalid",
        );
        assert_error(
            "(?<!hello)*",
            "Parsing error at position 10: Target of repeat operator is invalid",
        );
        assert_error(
            "(?<=hello){2,3}",
            "Parsing error at position 14: Target of repeat operator is invalid",
        );
        assert_error(
            "(?!hello)?",
            "Parsing error at position 9: Target of repeat operator is invalid",
        );
        assert_error(
            "(a|b|?)",
            "Parsing error at position 5: Target of repeat operator is invalid",
        );
    }

    #[test]
    fn no_quantifiers_on_assertions_in_oniguruma_mode() {
        assert_error_oniguruma(
            r"\b{1}",
            "Parsing error at position 4: Target of repeat operator is invalid",
        );
        assert_error_oniguruma(
            r"\B{2}",
            "Parsing error at position 4: Target of repeat operator is invalid",
        );
        assert_error_oniguruma(
            r"^{3}",
            "Parsing error at position 3: Target of repeat operator is invalid",
        );
        assert_error_oniguruma(
            r"${1,5}",
            "Parsing error at position 5: Target of repeat operator is invalid",
        );
        assert_error_oniguruma(
            r"\A*",
            "Parsing error at position 2: Target of repeat operator is invalid",
        );
        assert_error_oniguruma(
            r"\z+",
            "Parsing error at position 2: Target of repeat operator is invalid",
        );
        assert_error_oniguruma(
            r"^?",
            "Parsing error at position 1: Target of repeat operator is invalid",
        );
        assert_error_oniguruma(
            r"${2}",
            "Parsing error at position 3: Target of repeat operator is invalid",
        );
        assert_error_oniguruma(
            r"(?m)^?",
            "Parsing error at position 5: Target of repeat operator is invalid",
        );
        assert_error_oniguruma(
            r"(?m)${2}",
            "Parsing error at position 7: Target of repeat operator is invalid",
        );
        assert_error_oniguruma(
            r"\b+",
            "Parsing error at position 2: Target of repeat operator is invalid",
        );
    }

    #[test]
    fn keepout() {
        assert_eq!(
            p("a\\Kb"),
            Expr::Concat(vec![make_literal("a"), Expr::KeepOut, make_literal("b"),])
        );
    }

    #[test]
    fn no_quantifiers_on_other_non_repeatable_expressions() {
        assert_error(
            r"\K?",
            "Parsing error at position 2: Target of repeat operator is invalid",
        );
        assert_error(
            r"\G*",
            "Parsing error at position 2: Target of repeat operator is invalid",
        );
    }

    #[test]
    fn empty_noncapturing_group_with_quantifier_default_mode() {
        // In default mode, quantifiers on empty non-capturing groups are errors.
        assert_error(
            "(?:)*",
            "Parsing error at position 4: Target of repeat operator is invalid",
        );
        assert_error(
            "(?:)+",
            "Parsing error at position 4: Target of repeat operator is invalid",
        );
        assert_error(
            "(?:)?",
            "Parsing error at position 4: Target of repeat operator is invalid",
        );
        assert_error(
            "(?:){3}",
            "Parsing error at position 6: Target of repeat operator is invalid",
        );
    }

    #[test]
    fn empty_noncapturing_group_with_quantifier_oniguruma_mode() {
        // Oniguruma silently accepts quantifiers on empty non-capturing groups
        // as zero-width no-ops, collapsing them to Expr::Empty.
        assert_eq!(parse_oniguruma("(?:)*").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?:)+").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?:)?").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?:){3}").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?:){2,5}").unwrap(), Expr::Empty);
        // Non-greedy modifier
        assert_eq!(parse_oniguruma("(?:)*?").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?:)+?").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?:)??").unwrap(), Expr::Empty);
        // Possessive modifier
        assert_eq!(parse_oniguruma("(?:)*+").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?:)++").unwrap(), Expr::Empty);
        // Both non-greedy and possessive
        assert_eq!(parse_oniguruma("(?:)*?+").unwrap(), Expr::Empty);
    }

    #[test]
    fn start_line_in_oniguruma_mode() {
        assert_eq!(
            parse_oniguruma("(?R:^)").unwrap(),
            Expr::Assertion(Assertion::StartLineOniguruma { crlf: true })
        );
        assert_eq!(
            parse_oniguruma("^").unwrap(),
            Expr::Assertion(Assertion::StartLineOniguruma { crlf: false })
        );
    }

    #[test]
    fn empty_noncapturing_group_with_quantifier_extended_mode() {
        // In (?x) mode, whitespace between the group and quantifier (and
        // between the quantifier and its modifiers) should be handled
        // correctly.
        assert_eq!(parse_oniguruma("(?x: (?:) * )").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?x: (?:) + )").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?x: (?:) ? )").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?x: (?:) {3} )").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?x: (?:) {2,5} )").unwrap(), Expr::Empty);
        // Non-greedy modifier with whitespace
        assert_eq!(parse_oniguruma("(?x: (?:) * ? )").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?x: (?:) + ? )").unwrap(), Expr::Empty);
        // Possessive modifier with whitespace
        assert_eq!(parse_oniguruma("(?x: (?:) * + )").unwrap(), Expr::Empty);
    }

    #[test]
    fn empty_noncapturing_group_with_quantifier_in_concat() {
        // Empty group + quantifier in the middle of a larger pattern should
        // parse correctly, with the empty group collapsing away.
        assert_eq!(
            parse_oniguruma("a(?:)*b").unwrap(),
            Expr::Concat(vec![make_literal("a"), make_literal("b")])
        );
        assert_eq!(
            parse_oniguruma("(?x: a (?:) * b )").unwrap(),
            Expr::Concat(vec![make_literal("a"), make_literal("b")])
        );
    }

    #[test]
    fn empty_noncapturing_group_with_flags_and_quantifier_oniguruma_mode() {
        // Non-capturing groups with flags like (?i:) that are empty should
        // also be handled.
        assert_eq!(parse_oniguruma("(?i:)*").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?s:)+").unwrap(), Expr::Empty);
        assert_eq!(parse_oniguruma("(?x: (?i:) * )").unwrap(), Expr::Empty);
    }

    #[test]
    fn swapped_order_quantifier_oniguruma_mode() {
        assert_eq!(
            parse_oniguruma("x{3,0}").unwrap(),
            Expr::AtomicGroup(Box::new(Expr::Repeat {
                child: Box::new(make_literal("x")),
                lo: 0,
                hi: 3,
                greedy: true,
            }))
        );
    }

    #[test]
    fn plus_after_user_specified_quantifier_oniguruma_mode() {
        assert_eq!(
            parse_oniguruma("x{2}+").unwrap(),
            Expr::Repeat {
                child: Box::new(Expr::Repeat {
                    child: Box::new(make_literal("x")),
                    lo: 2,
                    hi: 2,
                    greedy: true,
                }),
                lo: 1,
                hi: usize::MAX,
                greedy: true,
            }
        );
    }

    #[test]
    fn backref_exists_condition() {
        assert_eq!(
            p("(h)?(?(1))"),
            Expr::Concat(vec![
                Expr::Repeat {
                    child: Box::new(make_group(make_literal("h"))),
                    lo: 0,
                    hi: 1,
                    greedy: true
                },
                Expr::BackrefExistsCondition {
                    group: 1,
                    relative_recursion_level: None
                }
            ])
        );
        assert_eq!(
            p("(?<h>h)?(?('h'))"),
            Expr::Concat(vec![
                Expr::Repeat {
                    child: Box::new(make_group(make_literal("h"))),
                    lo: 0,
                    hi: 1,
                    greedy: true
                },
                Expr::BackrefExistsCondition {
                    group: 1,
                    relative_recursion_level: None
                }
            ])
        );
    }

    #[test]
    fn backref_exists_condition_with_recursion_level_suffix() {
        // `(?(1+0)b|c)` is a conditional that tests whether group 1 matched at relative
        // recursion level 0.  The parser accepts it; compile rejects it as not yet supported.
        // Named forms require delimiters: `(?(<n+0>)...)` or `(?('n+0')...)`.
        for pattern in &[
            r"(a)(?(1+0)b|c)d",
            r"(?<n>a)(?(<n+0>)b|c)d",
            r"(?<n>a)(?('n+0')b|c)d",
        ] {
            assert_eq!(
                p(pattern),
                Expr::Concat(vec![
                    make_group(make_literal("a")),
                    Expr::Conditional {
                        condition: Box::new(Expr::BackrefExistsCondition {
                            group: 1,
                            relative_recursion_level: Some(0),
                        }),
                        true_branch: Box::new(make_literal("b")),
                        false_branch: Box::new(make_literal("c")),
                    },
                    make_literal("d"),
                ]),
                "pattern: {pattern:?}",
            );
        }
    }

    #[test]
    fn conditional_non_backref_validity_check_without_branches() {
        assert_error(
            "(?(foo))",
            "Parsing error at position 7: General parsing error: expected conditional to be a backreference or at least an expression for when the condition is true",
        );
    }

    #[test]
    fn conditional_invalid_target_of_repeat_operator() {
        assert_error(
            r"(?(?=\d)\w|!)",
            "Parsing error at position 3: Target of repeat operator is invalid",
        );
    }

    #[test]
    fn conditional_unclosed_at_end_of_pattern() {
        assert_error(
            r"(?(",
            "Parsing error at position 3: Opening parenthesis without closing parenthesis",
        );
    }

    #[test]
    fn subroutine_call_unclosed_at_end_of_pattern() {
        assert_error(
            r"\g<",
            "Parsing error at position 2: Could not parse group name",
        );

        assert_error(
            r"\g<name",
            "Parsing error at position 2: Could not parse group name",
        );

        assert_error(
            r"\g'",
            "Parsing error at position 2: Could not parse group name",
        );

        assert_error(
            r"\g''",
            "Parsing error at position 2: Could not parse group name",
        );

        assert_error(
            r"\g<>",
            "Parsing error at position 2: Could not parse group name",
        );

        assert_error(r"\g", "Parsing error at position 0: Invalid escape: \\g");

        assert_error(
            r"\g test",
            "Parsing error at position 2: Could not parse group name",
        );
    }

    #[test]
    fn subroutine_call_missing_subroutine_reference() {
        assert_error(
            r"\g test",
            "Parsing error at position 2: Could not parse group name",
        );
    }

    #[test]
    fn subroutine_call_name_includes_dash() {
        // With unrestricted names in subroutine calls, these are now valid names
        // (though they won't resolve to actual groups, that's caught during analysis)
        let tree = Expr::parse_tree(r"\g<1-0>(?<1-0>a)").unwrap();
        assert!(tree.contains_subroutines);

        let tree = Expr::parse_tree(r"\g<name+1>(?<name+1>a)").unwrap();
        assert!(tree.contains_subroutines);
    }

    #[test]
    fn backref_condition_with_one_two_or_three_branches() {
        assert_eq!(
            p("(h)?(?(1)i|x)"),
            Expr::Concat(vec![
                Expr::Repeat {
                    child: Box::new(make_group(make_literal("h"))),
                    lo: 0,
                    hi: 1,
                    greedy: true
                },
                Expr::Conditional {
                    condition: Box::new(Expr::BackrefExistsCondition {
                        group: 1,
                        relative_recursion_level: None
                    }),
                    true_branch: Box::new(make_literal("i")),
                    false_branch: Box::new(make_literal("x")),
                },
            ])
        );

        assert_eq!(
            p("(h)?(?(1)i)"),
            Expr::Concat(vec![
                Expr::Repeat {
                    child: Box::new(make_group(make_literal("h"))),
                    lo: 0,
                    hi: 1,
                    greedy: true
                },
                Expr::Conditional {
                    condition: Box::new(Expr::BackrefExistsCondition {
                        group: 1,
                        relative_recursion_level: None
                    }),
                    true_branch: Box::new(make_literal("i")),
                    false_branch: Box::new(Expr::Empty),
                },
            ])
        );

        assert_eq!(
            p("(h)?(?(1)ii|xy|z)"),
            Expr::Concat(vec![
                Expr::Repeat {
                    child: Box::new(make_group(make_literal("h"))),
                    lo: 0,
                    hi: 1,
                    greedy: true
                },
                Expr::Conditional {
                    condition: Box::new(Expr::BackrefExistsCondition {
                        group: 1,
                        relative_recursion_level: None
                    }),
                    true_branch: Box::new(Expr::Concat(
                        vec![make_literal("i"), make_literal("i"),]
                    )),
                    false_branch: Box::new(Expr::Alt(vec![
                        Expr::Concat(vec![make_literal("x"), make_literal("y"),]),
                        make_literal("z"),
                    ])),
                },
            ])
        );

        assert_eq!(
            p("(?<cap>h)?(?(<cap>)ii|xy|z)"),
            Expr::Concat(vec![
                Expr::Repeat {
                    child: Box::new(make_group(make_literal("h"))),
                    lo: 0,
                    hi: 1,
                    greedy: true
                },
                Expr::Conditional {
                    condition: Box::new(Expr::BackrefExistsCondition {
                        group: 1,
                        relative_recursion_level: None
                    }),
                    true_branch: Box::new(Expr::Concat(
                        vec![make_literal("i"), make_literal("i"),]
                    )),
                    false_branch: Box::new(Expr::Alt(vec![
                        Expr::Concat(vec![make_literal("x"), make_literal("y"),]),
                        make_literal("z"),
                    ])),
                },
            ])
        );
    }

    #[test]
    fn conditional() {
        assert_eq!(
            p("((?(a)b|c))(\\1)"),
            Expr::Concat(vec![
                make_group(Expr::Conditional {
                    condition: Box::new(make_literal("a")),
                    true_branch: Box::new(make_literal("b")),
                    false_branch: Box::new(make_literal("c"))
                }),
                make_group(Expr::Backref {
                    group: 1,
                    casei: false,
                },)
            ])
        );

        assert_eq!(
            p(r"^(?(\d)abc|\d!)$"),
            Expr::Concat(vec![
                Expr::Assertion(Assertion::StartText),
                Expr::Conditional {
                    condition: Box::new(Expr::Delegate {
                        inner: "\\d".to_string(),
                        casei: false,
                    }),
                    true_branch: Box::new(Expr::Concat(vec![
                        make_literal("a"),
                        make_literal("b"),
                        make_literal("c"),
                    ])),
                    false_branch: Box::new(Expr::Concat(vec![
                        Expr::Delegate {
                            inner: "\\d".to_string(),
                            casei: false,
                        },
                        make_literal("!"),
                    ])),
                },
                Expr::Assertion(Assertion::EndText),
            ])
        );

        assert_eq!(
            p(r"(?((?=\d))\w|!)"),
            Expr::Conditional {
                condition: Box::new(Expr::LookAround(
                    Box::new(Expr::Delegate {
                        inner: "\\d".to_string(),
                        casei: false,
                    }),
                    LookAhead
                )),
                true_branch: Box::new(Expr::Delegate {
                    inner: "\\w".to_string(),
                    casei: false,
                }),
                false_branch: Box::new(make_literal("!")),
            },
        );

        assert_eq!(
            p(r"(?((ab))c|d)"),
            Expr::Conditional {
                condition: Box::new(make_group(Expr::Concat(vec![
                    make_literal("a"),
                    make_literal("b"),
                ]),)),
                true_branch: Box::new(make_literal("c")),
                false_branch: Box::new(make_literal("d")),
            },
        );
    }

    #[test]
    fn subroutines() {
        assert_eq!(
            p(r"(a)\g1"),
            Expr::Concat(vec![make_group(make_literal("a")), Expr::SubroutineCall(1)])
        );

        assert_eq!(
            p(r"(a)\g<1>"),
            Expr::Concat(vec![make_group(make_literal("a")), Expr::SubroutineCall(1)])
        );

        assert_eq!(
            p(r"(?<group_name>a)\g<group_name>"),
            Expr::Concat(vec![make_group(make_literal("a")), Expr::SubroutineCall(1)])
        );

        assert_eq!(
            p(r"(?<group_name>a)\g'group_name'"),
            Expr::Concat(vec![make_group(make_literal("a")), Expr::SubroutineCall(1)])
        );

        assert_eq!(
            p(r"(?<group_name>a)(?P>group_name)"),
            Expr::Concat(vec![make_group(make_literal("a")), Expr::SubroutineCall(1)])
        );
    }

    #[test]
    fn subroutines_with_unrestricted_names() {
        // Hyphenated names via \g<name>
        assert_eq!(
            p(r"(?<foo-bar>a)\g<foo-bar>"),
            Expr::Concat(vec![make_group(make_literal("a")), Expr::SubroutineCall(1)])
        );

        // Hyphenated names via \g'name'
        assert_eq!(
            p(r"(?<foo-bar>a)\g'foo-bar'"),
            Expr::Concat(vec![make_group(make_literal("a")), Expr::SubroutineCall(1)])
        );

        // Hyphenated names via (?P>name)
        assert_eq!(
            p(r"(?<foo-bar>a)(?P>foo-bar)"),
            Expr::Concat(vec![make_group(make_literal("a")), Expr::SubroutineCall(1)])
        );

        // Forward reference with hyphenated name
        assert_eq!(
            p(r"\g<foo-bar>(?<foo-bar>a)"),
            Expr::Concat(vec![Expr::SubroutineCall(1), make_group(make_literal("a"))])
        );

        // Emoji name via \g
        assert_eq!(
            p("(?<\u{1F3AF}>a)\\g<\u{1F3AF}>"),
            Expr::Concat(vec![make_group(make_literal("a")), Expr::SubroutineCall(1)])
        );

        // Emoji name via (?P>)
        assert_eq!(
            p("(?<\u{1F3AF}>a)(?P>\u{1F3AF})"),
            Expr::Concat(vec![make_group(make_literal("a")), Expr::SubroutineCall(1)])
        );

        // Space in name via \g
        assert_eq!(
            p(r"(?<a b>x)\g<a b>"),
            Expr::Concat(vec![make_group(make_literal("x")), Expr::SubroutineCall(1)])
        );
    }

    #[test]
    fn subroutine_defined_later() {
        assert_eq!(
            p(r"\g<name>(?<name>a)"),
            Expr::Concat(vec![Expr::SubroutineCall(1), make_group(make_literal("a")),])
        );

        assert_eq!(
            p(r"\g<c>(?:a|b|(?<c>c)?)"),
            Expr::Concat(vec![
                Expr::SubroutineCall(1),
                Expr::Alt(vec![
                    make_literal("a"),
                    make_literal("b"),
                    Expr::Repeat {
                        child: Box::new(make_group(make_literal("c"))),
                        lo: 0,
                        hi: 1,
                        greedy: true
                    }
                ])
            ])
        );

        assert_eq!(
            p(r"(?<a>a)?\g<b>(?(<a>)(?<b>b)|c)"),
            Expr::Concat(vec![
                Expr::Repeat {
                    child: Box::new(make_group(make_literal("a"))),
                    lo: 0,
                    hi: 1,
                    greedy: true
                },
                Expr::SubroutineCall(2),
                Expr::Conditional {
                    condition: Box::new(Expr::BackrefExistsCondition {
                        group: 1,
                        relative_recursion_level: None
                    }),
                    true_branch: Box::new(make_group(make_literal("b"))),
                    false_branch: Box::new(make_literal("c")),
                }
            ])
        );

        assert_eq!(
            p(r"\g<1>(a)"),
            Expr::Concat(vec![Expr::SubroutineCall(1), make_group(make_literal("a")),])
        );
    }

    #[test]
    fn recursive_subroutine_call() {
        assert_eq!(
            p(r"\A(?<a>|.|(?:(?<b>.)\g<a>\k<b>))\z"),
            Expr::Concat(vec![
                Expr::Assertion(Assertion::StartText,),
                make_group(Expr::Alt(vec![
                    Expr::Empty,
                    Expr::Any {
                        newline: false,
                        crlf: false
                    },
                    Expr::Concat(vec![
                        make_group(Expr::Any {
                            newline: false,
                            crlf: false
                        },),
                        Expr::SubroutineCall(1,),
                        Expr::Backref {
                            group: 2,
                            casei: false,
                        },
                    ],),
                ],),),
                Expr::Assertion(Assertion::EndText,),
            ],)
        );
    }

    #[test]
    fn self_recursive_subroutine_call() {
        let tree = Expr::parse_tree(r"hello\g<0>?world").unwrap();
        assert!(tree.self_recursive);
        assert!(tree.numeric_capture_group_references);

        let tree = Expr::parse_tree(r"hello\g0?world").unwrap();
        assert!(tree.self_recursive);
        assert!(tree.numeric_capture_group_references);

        let tree = Expr::parse_tree(r"hello world").unwrap();
        assert!(!tree.self_recursive);
        assert!(!tree.numeric_capture_group_references);

        let tree = Expr::parse_tree(r"hello\g1world").unwrap();
        assert!(!tree.self_recursive);
        assert!(tree.numeric_capture_group_references);

        let tree = Expr::parse_tree(r"hello\g<1>world").unwrap();
        assert!(!tree.self_recursive);
        assert!(tree.numeric_capture_group_references);

        let tree = Expr::parse_tree(r"(hello\g1?world)").unwrap();
        assert!(!tree.self_recursive);
        assert!(tree.numeric_capture_group_references);

        let tree = Expr::parse_tree(r"(?<a>hello\g<a>world)").unwrap();
        assert!(!tree.self_recursive);
        assert!(!tree.numeric_capture_group_references);
    }

    // found by cargo fuzz, then minimized
    #[test]
    fn fuzz_1() {
        p(r"\ä");
    }

    #[test]
    fn fuzz_2() {
        p(r"\pä");
    }

    #[test]
    fn fuzz_3() {
        fail(r"(?()^");
        fail(r#"!w(?()\"Kuz>"#);
    }

    #[test]
    fn fuzz_4() {
        fail(r"\u{2}(?(2)");
    }

    #[test]
    fn word_boundary_brace_syntax() {
        // Valid patterns produce correct assertions
        assert_eq!(
            p(r"\b{start}"),
            Expr::Assertion(Assertion::LeftWordBoundary)
        );
        assert_eq!(p(r"\b{end}"), Expr::Assertion(Assertion::RightWordBoundary));
        assert_eq!(
            p(r"\b{start-half}"),
            Expr::Assertion(Assertion::LeftWordHalfBoundary)
        );
        assert_eq!(
            p(r"\b{end-half}"),
            Expr::Assertion(Assertion::RightWordHalfBoundary)
        );
    }

    #[test]
    fn word_boundary_brace_parsing_errors() {
        // Invalid patterns produce expected errors
        let test_cases = [
            (r"\b{invalid}", "Invalid escape: \\b{invalid}"),
            (r"\b{start ", "Invalid escape: \\b{...}"),
            (r"\b{end ", "Invalid escape: \\b{...}"),
            (r"\b{}", "Invalid escape: \\b{}"),
            (r"\b{", "Invalid escape: \\b{...}"),
            (r"\b{ }", "Invalid escape: \\b{ }"),
            (r"\b{START}", "Invalid escape: \\b{START}"),
            (r"\b{END}", "Invalid escape: \\b{END}"),
            (r"\b{ s t a r t }", "Invalid escape: \\b{ s t a r t }"),
            // \B{...} patterns should fail
            (r"\B{start}", "Invalid escape: \\B{start}"),
            (r"\B{end}", "Invalid escape: \\B{end}"),
            (r"\B{start-half}", "Invalid escape: \\B{start-half}"),
            (r"\B{end-half}", "Invalid escape: \\B{end-half}"),
            (r"\c{start}", "Invalid escape: \\c"),
        ];

        for (pattern, expected_error) in test_cases {
            assert_error(
                pattern,
                &format!("Parsing error at position 0: {}", expected_error),
            );
        }
    }

    fn get_options(func: impl Fn(SyntaxConfig) -> SyntaxConfig) -> RegexOptions {
        let mut options = RegexOptions::default();
        options.syntaxc = func(options.syntaxc);
        options
    }

    #[test]
    fn parse_with_case_insensitive_in_pattern() {
        let tree = Expr::parse_tree("(?i)hello");
        let expr = tree.unwrap().expr;

        assert_eq!(
            expr,
            Expr::Concat(vec![
                make_literal_case_insensitive("h", true),
                make_literal_case_insensitive("e", true),
                make_literal_case_insensitive("l", true),
                make_literal_case_insensitive("l", true),
                make_literal_case_insensitive("o", true)
            ])
        );
    }

    #[test]
    fn parse_with_case_insensitive_option() {
        let options = get_options(|x| x.case_insensitive(true));

        let tree = Expr::parse_tree_with_flags("hello", options.compute_flags());
        let expr = tree.unwrap().expr;

        assert_eq!(
            expr,
            Expr::Concat(vec![
                make_literal_case_insensitive("h", true),
                make_literal_case_insensitive("e", true),
                make_literal_case_insensitive("l", true),
                make_literal_case_insensitive("l", true),
                make_literal_case_insensitive("o", true)
            ])
        );
    }

    #[test]
    fn parse_with_multiline_in_pattern() {
        let options = get_options(|x| x);

        let tree = Expr::parse_tree_with_flags("(?m)^hello$", options.compute_flags());
        let expr = tree.unwrap().expr;

        assert_eq!(
            expr,
            Expr::Concat(vec![
                Expr::Assertion(Assertion::StartLine { crlf: false }),
                make_literal("h"),
                make_literal("e"),
                make_literal("l"),
                make_literal("l"),
                make_literal("o"),
                Expr::Assertion(Assertion::EndLine { crlf: false })
            ])
        );
    }

    #[test]
    fn parse_with_multiline_option() {
        let options = get_options(|x| x.multi_line(true));

        let tree = Expr::parse_tree_with_flags("^hello$", options.compute_flags());
        let expr = tree.unwrap().expr;

        assert_eq!(
            expr,
            Expr::Concat(vec![
                Expr::Assertion(Assertion::StartLine { crlf: false }),
                make_literal("h"),
                make_literal("e"),
                make_literal("l"),
                make_literal("l"),
                make_literal("o"),
                Expr::Assertion(Assertion::EndLine { crlf: false })
            ])
        );
    }

    #[test]
    fn parse_with_dot_matches_new_line_in_pattern() {
        let options = get_options(|x| x);

        let tree = Expr::parse_tree_with_flags("(?s)(.*)", options.compute_flags());
        let expr = tree.unwrap().expr;

        assert_eq!(
            expr,
            make_group(Expr::Repeat {
                child: Box::new(Expr::Any {
                    newline: true,
                    crlf: false
                }),
                lo: 0,
                hi: usize::MAX,
                greedy: true
            })
        );
    }

    #[test]
    fn parse_with_dot_matches_new_line_option() {
        let options = get_options(|x| x.dot_matches_new_line(true));

        let tree = Expr::parse_tree_with_flags("(.*)", options.compute_flags());
        let expr = tree.unwrap().expr;

        assert_eq!(
            expr,
            make_group(Expr::Repeat {
                child: Box::new(Expr::Any {
                    newline: true,
                    crlf: false
                }),
                lo: 0,
                hi: usize::MAX,
                greedy: true
            })
        );
    }

    #[test]
    fn parse_fancy_with_dot_matches_new_line_in_pattern() {
        let options = get_options(|x| x.dot_matches_new_line(true));

        let tree = Expr::parse_tree_with_flags("(.*)(?<=hugo)", options.compute_flags());
        let expr = tree.unwrap().expr;

        assert_eq!(
            expr,
            Expr::Concat(vec![
                make_group(Expr::Repeat {
                    child: Box::new(Expr::Any {
                        newline: true,
                        crlf: false
                    }),
                    lo: 0,
                    hi: usize::MAX,
                    greedy: true
                }),
                Expr::LookAround(
                    Box::new(Expr::Concat(vec![
                        make_literal("h"),
                        make_literal("u"),
                        make_literal("g"),
                        make_literal("o")
                    ])),
                    LookBehind
                )
            ])
        );
    }

    #[test]
    fn parse_with_case_insensitive_from_pattern_and_multi_line_option() {
        let options = get_options(|x| x.multi_line(true));

        let tree = Expr::parse_tree_with_flags("(?i)^hello$", options.compute_flags());
        let expr = tree.unwrap().expr;

        assert_eq!(
            expr,
            Expr::Concat(vec![
                Expr::Assertion(Assertion::StartLine { crlf: false }),
                make_literal_case_insensitive("h", true),
                make_literal_case_insensitive("e", true),
                make_literal_case_insensitive("l", true),
                make_literal_case_insensitive("l", true),
                make_literal_case_insensitive("o", true),
                Expr::Assertion(Assertion::EndLine { crlf: false })
            ])
        );
    }

    #[test]
    fn parse_with_multi_line_and_case_insensitive_options() {
        let mut options = get_options(|x| x.multi_line(true));
        options.syntaxc = options.syntaxc.case_insensitive(true);

        let tree = Expr::parse_tree_with_flags("^hello$", options.compute_flags());
        let expr = tree.unwrap().expr;

        assert_eq!(
            expr,
            Expr::Concat(vec![
                Expr::Assertion(Assertion::StartLine { crlf: false }),
                make_literal_case_insensitive("h", true),
                make_literal_case_insensitive("e", true),
                make_literal_case_insensitive("l", true),
                make_literal_case_insensitive("l", true),
                make_literal_case_insensitive("o", true),
                Expr::Assertion(Assertion::EndLine { crlf: false })
            ])
        );
    }

    #[test]
    fn parse_backtracking_control_verbs() {
        assert_eq!(
            p(r"(*FAIL)"),
            Expr::BacktrackingControlVerb(BacktrackingControlVerb::Fail)
        );
        assert_eq!(
            p(r"(*F)"),
            Expr::BacktrackingControlVerb(BacktrackingControlVerb::Fail)
        );
        assert_eq!(
            p(r"a(*FAIL)"),
            Expr::Concat(vec![
                make_literal("a"),
                Expr::BacktrackingControlVerb(BacktrackingControlVerb::Fail)
            ])
        );
        assert_eq!(
            p(r"(?(*FAIL)a|b)"),
            Expr::Conditional {
                condition: Box::new(Expr::BacktrackingControlVerb(BacktrackingControlVerb::Fail)),
                true_branch: Box::new(make_literal("a")),
                false_branch: Box::new(make_literal("b"))
            }
        );

        assert_eq!(
            p(r"(*ACCEPT)"),
            Expr::BacktrackingControlVerb(BacktrackingControlVerb::Accept)
        );
        assert_eq!(
            p(r"(*COMMIT)"),
            Expr::BacktrackingControlVerb(BacktrackingControlVerb::Commit)
        );
        assert_eq!(
            p(r"(*SKIP)"),
            Expr::BacktrackingControlVerb(BacktrackingControlVerb::Skip)
        );
        assert_eq!(
            p(r"(*PRUNE)"),
            Expr::BacktrackingControlVerb(BacktrackingControlVerb::Prune)
        );

        assert_error(
            "(*RANDOM)",
            "Parsing error at position 1: Target of repeat operator is invalid",
        );
    }

    #[test]
    fn remap_unicode_property_if_necessary_outside_class_tests() {
        // Test \p with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{alnum}", true, false),
            r"[\p{alpha}\p{digit}]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{blank}", true, false),
            r"[\p{Zs}\x09]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{word}", true, false),
            r"\w"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{Greek}", true, false),
            r"\p{greek}"
        );

        // Test \p without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{alnum}", false, false),
            r"[[:alnum:]]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{blank}", false, false),
            r"[\t ]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{word}", false, false),
            r"[_[:alnum:]]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{Greek}", false, false),
            r"\p{greek}"
        );

        // Test \P with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{alnum}", true, false),
            r"[^\p{alpha}\p{digit}]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{blank}", true, false),
            r"[^\p{Zs}\x09]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{word}", true, false),
            r"\W"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{Greek}", true, false),
            r"\P{greek}"
        );

        // Test \P without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{alnum}", false, false),
            r"[^[:alnum:]]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{blank}", false, false),
            r"[^\t ]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{word}", false, false),
            r"[^_[:alnum:]]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{Greek}", false, false),
            r"\P{greek}"
        );

        // Test \p{cntrl} with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{cntrl}", true, false),
            r"[\x00-\x1F\x7F-\x9F]"
        );
        // Test \P{cntrl} with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{cntrl}", true, false),
            r"[^\x00-\x1F\x7F-\x9F]"
        );
        // Test \p{cntrl} without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{cntrl}", false, false),
            r"[[:cntrl:]]"
        );
        // Test \P{cntrl} without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{cntrl}", false, false),
            r"[^[:cntrl:]]"
        );

        // Test \p{graph} with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{graph}", true, false),
            r"[^\p{White_Space}\p{C}]"
        );
        // Test \P{graph} with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{graph}", true, false),
            r"[\p{White_Space}\p{C}]"
        );
        // Test \p{graph} without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{graph}", false, false),
            r"[[:graph:]]"
        );
        // Test \P{graph} without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{graph}", false, false),
            r"[^[:graph:]]"
        );

        // Test \p{print} with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{print}", true, false),
            r"[^\p{C}\t\n\v\f\r]"
        );
        // Test \P{print} with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{print}", true, false),
            r"[\p{C}\t\n\v\f\r]"
        );
        // Test \p{print} without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{print}", false, false),
            r"[[:print:]]"
        );
        // Test \P{print} without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{print}", false, false),
            r"[^[:print:]]"
        );
        // Test \p{cs} and \P{cs} (surrogates, never matches in UTF-8)
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{Cs}", true, false),
            r"\P{any}"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{Cs}", true, false),
            r"\p{any}"
        );
    }

    #[test]
    fn remap_unicode_property_if_necessary_inside_class_tests() {
        // Test \p with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{alnum}", true, true),
            r"\p{alpha}\p{digit}"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{blank}", true, true),
            r"\p{Zs}\x09"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{word}", true, true),
            r"\w"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{Greek}", true, true),
            r"\p{greek}"
        );

        // Test \p without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{alnum}", false, true),
            r"[:alnum:]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{blank}", false, true),
            r"\t "
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{word}", false, true),
            r"_[:alnum:]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{Greek}", false, true),
            r"\p{greek}"
        );

        // Test \P with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{alnum}", true, true),
            r"[^\p{alpha}\p{digit}]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{blank}", true, true),
            r"[^\p{Zs}\x09]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{word}", true, true),
            r"\W"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{Greek}", true, true),
            r"\P{greek}"
        );

        // Test \p{cntrl} inside class with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{cntrl}", true, true),
            r"\x00-\x1F\x7F-\x9F"
        );
        // Test \P{cntrl} inside class with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{cntrl}", true, true),
            r"[^\x00-\x1F\x7F-\x9F]"
        );
        // Test \p{cntrl} inside class without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{cntrl}", false, true),
            r"[:cntrl:]"
        );
        // Test \P{cntrl} inside class without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{cntrl}", false, true),
            r"[^[:cntrl:]]"
        );

        // Test \p{graph} inside class with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{graph}", true, true),
            r"[^\p{White_Space}\p{C}]"
        );
        // Test \P{graph} inside class with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{graph}", true, true),
            r"[\p{White_Space}\p{C}]"
        );
        // Test \p{graph} inside class without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{graph}", false, true),
            r"[:graph:]"
        );
        // Test \P{graph} inside class without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{graph}", false, true),
            r"[^[:graph:]]"
        );

        // Test \p{print} inside class with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{print}", true, true),
            r"[^\p{C}\t\n\v\f\r]"
        );
        // Test \P{print} inside class with unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{print}", true, true),
            r"[\p{C}\t\n\v\f\r]"
        );
        // Test \p{print} inside class without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{print}", false, true),
            r"[:print:]"
        );
        // Test \P{print} inside class without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{print}", false, true),
            r"[^[:print:]]"
        );

        // Test \P without unicode flag
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{alnum}", false, true),
            r"[^[:alnum:]]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{blank}", false, true),
            r"[^\t ]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{word}", false, true),
            r"[^_[:alnum:]]"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{Greek}", false, true),
            r"\P{greek}"
        );
    }

    #[test]
    fn remap_unicode_property_case_insensitive() {
        // Test case-insensitive conversion for Emoji and other properties
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{Emoji}", true, false),
            r"\p{emoji}"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{EMOJI}", true, false),
            r"\p{emoji}"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{Emoji}", true, false),
            r"\P{emoji}"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{EMOJI}", true, false),
            r"\P{emoji}"
        );

        // Test Word property (both uppercase and lowercase)
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{Word}", true, false),
            r"\w"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{WORD}", true, false),
            r"\w"
        );
    }

    #[test]
    fn remap_unicode_property_caret_negation() {
        // Test \p{^...} syntax (Oniguruma negation inside braces)
        // \p{^Emoji} should become \P{emoji}
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{^Emoji}", true, false),
            r"\P{emoji}"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{^EMOJI}", true, false),
            r"\P{emoji}"
        );

        // \P{^Word} should become \p{word} which becomes \w
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{^Word}", true, false),
            r"\w"
        );
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{^WORD}", true, false),
            r"\w"
        );

        // \p{^Greek} should become \P{greek}
        assert_eq!(
            remap_unicode_property_if_necessary(r"\p{^Greek}", true, false),
            r"\P{greek}"
        );

        // \P{^Greek} should become \p{greek}
        assert_eq!(
            remap_unicode_property_if_necessary(r"\P{^Greek}", true, false),
            r"\p{greek}"
        );
    }

    #[test]
    fn parse_absent_repeater() {
        assert_eq!(
            p(r"(?~abc)"),
            Expr::Absent(Absent::Repeater(Box::new(Expr::Concat(vec![
                make_literal("a"),
                make_literal("b"),
                make_literal("c"),
            ]))))
        );
    }

    #[test]
    fn parse_absent_expression() {
        assert_eq!(
            p(r"(?~|abc|\d+)"),
            Expr::Absent(Absent::Expression {
                absent: Box::new(Expr::Concat(vec![
                    make_literal("a"),
                    make_literal("b"),
                    make_literal("c"),
                ])),
                exp: Box::new(Expr::Repeat {
                    child: Box::new(Expr::Delegate {
                        inner: "\\d".to_string(),
                        casei: false,
                    }),
                    lo: 1,
                    hi: usize::MAX,
                    greedy: true
                }),
            })
        );
    }

    #[test]
    fn parse_absent_stopper() {
        assert_eq!(
            p(r"(?~|abc)"),
            Expr::Absent(Absent::Stopper(Box::new(Expr::Concat(vec![
                make_literal("a"),
                make_literal("b"),
                make_literal("c"),
            ]))))
        );
    }

    #[test]
    fn parse_absent_range_clear() {
        assert_eq!(p(r"(?~|)"), Expr::Absent(Absent::Clear));
    }

    #[test]
    fn define_group() {
        assert_eq!(
            p(r"(?(DEFINE)(?<word>\w+))"),
            Expr::DefineGroup {
                definitions: Box::new(make_group(Expr::Repeat {
                    child: Box::new(Expr::Delegate {
                        inner: "\\w".to_string(),
                        casei: false,
                    }),
                    lo: 1,
                    hi: usize::MAX,
                    greedy: true,
                })),
            }
        );
    }

    #[test]
    fn define_group_with_subroutine_call() {
        assert_eq!(
            p(r"(?(DEFINE)(?<word>\w+))\g<word>"),
            Expr::Concat(vec![
                Expr::DefineGroup {
                    definitions: Box::new(make_group(Expr::Repeat {
                        child: Box::new(Expr::Delegate {
                            inner: "\\w".to_string(),
                            casei: false,
                        }),
                        lo: 1,
                        hi: usize::MAX,
                        greedy: true,
                    })),
                },
                Expr::SubroutineCall(1),
            ])
        );
    }
}
