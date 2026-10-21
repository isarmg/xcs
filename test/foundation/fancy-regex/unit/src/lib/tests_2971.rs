#[cfg(test)]
mod tests {
    use alloc::borrow::Cow;
    use alloc::boxed::Box;
    use alloc::string::{String, ToString};
    use alloc::sync::Arc;
    use alloc::vec::Vec;
    use alloc::{format, vec};

    use crate::parse::{make_group, make_literal};
    use crate::{Absent, Expr, Regex, RegexBuilder, RegexImpl, RegexInput};

    //use detect_possible_backref;

    // tests for to_str

    fn to_str(e: Expr) -> String {
        let mut s = String::new();
        e.to_str(&mut s, 0);
        s
    }

    #[test]
    fn to_str_concat_alt() {
        let e = Expr::Concat(vec![
            Expr::Alt(vec![make_literal("a"), make_literal("b")]),
            make_literal("c"),
        ]);
        assert_eq!(to_str(e), "(?:a|b)c");
    }

    #[test]
    fn to_str_rep_concat() {
        let e = Expr::Repeat {
            child: Box::new(Expr::Concat(vec![make_literal("a"), make_literal("b")])),
            lo: 2,
            hi: 3,
            greedy: true,
        };
        assert_eq!(to_str(e), "(?:ab){2,3}");
    }

    #[test]
    fn to_str_group_alt() {
        let e = Expr::Group(Arc::new(Expr::Alt(vec![
            make_literal("a"),
            make_literal("b"),
        ])));
        assert_eq!(to_str(e), "(a|b)");
    }

    #[test]
    fn to_str_literal_special_chars() {
        assert_eq!(to_str(make_literal("\n")), "\\n");
        assert_eq!(to_str(make_literal("\t")), "\\t");
        assert_eq!(to_str(make_literal("\r")), "\\r");
        assert_eq!(to_str(make_literal("\\")), "\\\\");
        assert_eq!(to_str(make_literal(".")), "\\.");
    }

    #[test]
    fn as_str_debug() {
        let s = r"(a+)b\1";
        let regex = Regex::new(s).unwrap();
        assert_eq!(s, regex.as_str());
        assert_eq!(s, format!("{:?}", regex));
    }

    #[test]
    fn display() {
        let s = r"(a+)b\1";
        let regex = Regex::new(s).unwrap();
        assert_eq!(s, format!("{}", regex));
    }

    #[test]
    fn from_str() {
        let s = r"(a+)b\1";
        let regex = s.parse::<Regex>().unwrap();
        assert_eq!(regex.as_str(), s);
    }

    #[test]
    fn to_str_repeat() {
        fn repeat(lo: usize, hi: usize, greedy: bool) -> Expr {
            Expr::Repeat {
                child: Box::new(make_literal("a")),
                lo,
                hi,
                greedy,
            }
        }

        assert_eq!(to_str(repeat(2, 2, true)), "a{2}");
        assert_eq!(to_str(repeat(2, 2, false)), "a{2}?");
        assert_eq!(to_str(repeat(2, 3, true)), "a{2,3}");
        assert_eq!(to_str(repeat(2, 3, false)), "a{2,3}?");
        assert_eq!(to_str(repeat(2, usize::MAX, true)), "a{2,}");
        assert_eq!(to_str(repeat(2, usize::MAX, false)), "a{2,}?");
        assert_eq!(to_str(repeat(0, 1, true)), "a?");
        assert_eq!(to_str(repeat(0, 1, false)), "a??");
        assert_eq!(to_str(repeat(0, usize::MAX, true)), "a*");
        assert_eq!(to_str(repeat(0, usize::MAX, false)), "a*?");
        assert_eq!(to_str(repeat(1, usize::MAX, true)), "a+");
        assert_eq!(to_str(repeat(1, usize::MAX, false)), "a+?");
    }

    #[test]
    fn escape() {
        // Check that strings that need no quoting are borrowed, and that non-special punctuation
        // is not quoted.
        match crate::escape("@foo") {
            Cow::Borrowed(s) => assert_eq!(s, "@foo"),
            _ => panic!("Value should be borrowed."),
        }

        // Check typical usage.
        assert_eq!(crate::escape("fo*o").into_owned(), "fo\\*o");

        // Check that multibyte characters are handled correctly.
        assert_eq!(crate::escape("fø*ø").into_owned(), "fø\\*ø");
    }

    #[test]
    fn push_quoted_special_chars() {
        fn pq(s: &str) -> String {
            let mut buf = String::new();
            crate::push_quoted(&mut buf, s);
            buf
        }
        assert_eq!(pq("\n"), "\\n");
        assert_eq!(pq("\t"), "\\t");
        assert_eq!(pq("\r"), "\\r");
        assert_eq!(pq("\\"), "\\\\");
        assert_eq!(pq("."), "\\.");
        assert_eq!(pq("hello"), "hello");
        assert_eq!(pq("a.b"), "a\\.b");
        assert_eq!(pq("fø*ø"), "fø\\*ø");
    }

    #[test]
    fn trailing_positive_lookahead_wrap_capture_group_fixup() {
        let s = r"a+(?=c)";
        let regex = s.parse::<Regex>().unwrap();
        assert!(matches!(regex.inner,
            RegexImpl::Wrap { explicit_capture_group_0: true, .. }),
            "trailing positive lookahead for an otherwise easy pattern should avoid going through the VM");
        assert_eq!(s, regex.as_str());
        assert_eq!(s, format!("{:?}", regex));
    }

    #[test]
    fn easy_regex() {
        let s = r"(a+)b";
        let regex = s.parse::<Regex>().unwrap();
        assert!(
            matches!(regex.inner, RegexImpl::Wrap { explicit_capture_group_0: false, .. }),
            "easy pattern should avoid going through the VM, and capture group 0 should be implicit"
        );

        assert_eq!(s, regex.as_str());
        assert_eq!(s, format!("{:?}", regex));
    }

    #[test]
    fn hard_regex() {
        let s = r"(a+)(?>c)";
        let regex = s.parse::<Regex>().unwrap();
        assert!(
            matches!(regex.inner, RegexImpl::Fancy { .. }),
            "hard regex should be compiled into a VM"
        );
        assert_eq!(s, regex.as_str());
        assert_eq!(s, format!("{:?}", regex));
    }

    #[cfg(feature = "leftmost_longest")]
    #[test]
    fn leftmost_longest_alt_compiles_to_fancy() {
        let regex = RegexBuilder::new(r"a|ab")
            .leftmost_longest(true)
            .build()
            .unwrap();
        assert!(
            matches!(regex.inner, RegexImpl::Fancy { .. }),
            "alternation with non-const size should compile to VM when leftmost_longest is enabled"
        );
    }

    #[test]
    fn start_end_text_assertions_can_stay_wrap_without_override_opt_in() {
        let regex = Regex::new(r"\Afoo\z").unwrap();
        assert!(
            matches!(regex.inner, RegexImpl::Wrap { .. }),
            r"\A...\z should stay on the wrap path unless input assertion overrides are enabled"
        );
    }

    #[test]
    fn start_end_text_assertions_become_fancy_with_override_opt_in() {
        let regex = RegexBuilder::new(r"\Afoo\z")
            .allow_input_assertion_overrides(true)
            .build()
            .unwrap();
        assert!(
            matches!(regex.inner, RegexImpl::Fancy { .. }),
            r"\A...\z should use the VM when input assertion overrides are enabled"
        );
    }

    /*
    #[test]
    fn detect_backref() {
        assert_eq!(detect_possible_backref("a0a1a2"), false);
        assert_eq!(detect_possible_backref("a0a1\\a2"), false);
        assert_eq!(detect_possible_backref("a0a\\1a2"), true);
        assert_eq!(detect_possible_backref("a0a1a2\\"), false);
    }
    */

    #[test]
    fn test_is_leaf_node_leaf_nodes() {
        // Test all leaf node variants
        assert!(Expr::Empty.is_leaf_node());
        assert!(Expr::Any {
            newline: false,
            crlf: false
        }
        .is_leaf_node());
        assert!(Expr::Any {
            newline: true,
            crlf: false
        }
        .is_leaf_node());
        assert!(Expr::Assertion(crate::Assertion::StartText).is_leaf_node());
        assert!(Expr::Literal {
            val: "test".to_string(),
            casei: false
        }
        .is_leaf_node());
        assert!(Expr::Delegate {
            inner: "[0-9]".to_string(),
            casei: false,
        }
        .is_leaf_node());
        assert!(Expr::Backref {
            group: 1,
            casei: false
        }
        .is_leaf_node());
        assert!(Expr::BackrefWithRelativeRecursionLevel {
            group: 1,
            relative_level: -1,
            casei: false
        }
        .is_leaf_node());
        assert!(Expr::KeepOut.is_leaf_node());
        assert!(Expr::ContinueFromPreviousMatchEnd.is_leaf_node());
        assert!(Expr::BackrefExistsCondition {
            group: 1,
            relative_recursion_level: None
        }
        .is_leaf_node());
        assert!(Expr::BacktrackingControlVerb(crate::BacktrackingControlVerb::Fail).is_leaf_node());
        assert!(Expr::SubroutineCall(1).is_leaf_node());

        assert!(Expr::Absent(Absent::Clear).is_leaf_node());
    }

    #[test]
    fn test_is_leaf_node_non_leaf_nodes() {
        // Test all non-leaf node variants
        assert!(!Expr::Concat(vec![make_literal("a")]).is_leaf_node());
        assert!(!Expr::Alt(vec![make_literal("a"), make_literal("b")]).is_leaf_node());
        assert!(!make_group(make_literal("a")).is_leaf_node());
        assert!(
            !Expr::LookAround(Box::new(make_literal("a")), crate::LookAround::LookAhead)
                .is_leaf_node()
        );
        assert!(!Expr::Repeat {
            child: Box::new(make_literal("a")),
            lo: 0,
            hi: 1,
            greedy: true
        }
        .is_leaf_node());
        assert!(!Expr::AtomicGroup(Box::new(make_literal("a"))).is_leaf_node());
        assert!(!Expr::Conditional {
            condition: Box::new(Expr::BackrefExistsCondition {
                group: 1,
                relative_recursion_level: None
            }),
            true_branch: Box::new(make_literal("a")),
            false_branch: Box::new(Expr::Empty)
        }
        .is_leaf_node());

        assert!(!Expr::Absent(Absent::Repeater(Box::new(make_literal("a")))).is_leaf_node());
        assert!(!Expr::Absent(Absent::Expression {
            absent: Box::new(make_literal("/*")),
            exp: Box::new(Expr::Repeat {
                child: Box::new(Expr::Any {
                    newline: true,
                    crlf: false
                }),
                lo: 0,
                hi: usize::MAX,
                greedy: true
            })
        })
        .is_leaf_node());
        assert!(!Expr::Absent(Absent::Stopper(Box::new(make_literal("/*")))).is_leaf_node());
    }

    #[test]
    fn test_children_iter_empty() {
        // Leaf nodes should return empty iterator
        let expr = Expr::Empty;
        let mut iter = expr.children_iter();
        assert!(iter.next().is_none());

        let expr = make_literal("test");
        let mut iter = expr.children_iter();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_children_iter_single() {
        // Group, LookAround, AtomicGroup, Repeat should return single child
        let child = make_literal("a");
        let expr = make_group(child.clone());
        let children: Vec<_> = expr.children_iter().collect();
        assert_eq!(children.len(), 1);

        let expr = Expr::Repeat {
            child: Box::new(child.clone()),
            lo: 0,
            hi: 1,
            greedy: true,
        };
        let children: Vec<_> = expr.children_iter().collect();
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn test_children_iter_vec() {
        // Concat and Alt should return all children
        let children_vec = vec![make_literal("a"), make_literal("b"), make_literal("c")];
        let expr = Expr::Concat(children_vec.clone());
        let children: Vec<_> = expr.children_iter().collect();
        assert_eq!(children.len(), 3);

        let expr = Expr::Alt(children_vec);
        let children: Vec<_> = expr.children_iter().collect();
        assert_eq!(children.len(), 3);
    }

    #[test]
    fn test_children_iter_triple() {
        // Conditional should return three children
        let expr = Expr::Conditional {
            condition: Box::new(Expr::BackrefExistsCondition {
                group: 1,
                relative_recursion_level: None,
            }),
            true_branch: Box::new(make_literal("a")),
            false_branch: Box::new(make_literal("b")),
        };
        let children: Vec<_> = expr.children_iter().collect();
        assert_eq!(children.len(), 3);

        // Absent expression should return two children
        let expr = Expr::Absent(Absent::Expression {
            absent: Box::new(make_literal("/*")),
            exp: Box::new(Expr::Repeat {
                child: Box::new(Expr::Any {
                    newline: true,
                    crlf: false,
                }),
                lo: 0,
                hi: usize::MAX,
                greedy: true,
            }),
        });
        let children: Vec<_> = expr.children_iter().collect();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn find_input_raw_honors_anchored_flag_for_wrapped_regex() {
        let regex = Regex::new("abc").unwrap();

        // Unanchored search finds match at position 1
        let input = RegexInput::new("zabc");
        assert_eq!(
            Some((1, 4)),
            regex
                .find_input(input)
                .unwrap()
                .map(|m| (m.start(), m.end()))
        );

        // Anchored at position 0 returns None (abc doesn't match at pos 0)
        let anchored_at_start = RegexInput::new("zabc").anchored(true);
        assert!(regex.find_input(anchored_at_start).unwrap().is_none());

        // Anchored at position 1 finds match
        let anchored_at_match = RegexInput::new("zabc").from_pos(1).anchored(true);
        assert_eq!(
            regex
                .find_input(anchored_at_match)
                .unwrap()
                .map(|m| (m.start(), m.end())),
            Some((1, 4))
        );
    }
}
