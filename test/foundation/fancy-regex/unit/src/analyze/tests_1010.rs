#[cfg(test)]
mod tests {
    use super::{analyze, AnalyzeContext};
    // use super::literal_const_size;
    use crate::parse::ExprTree;
    use crate::{can_compile_as_anchored, CompileError, Error, Expr};
    use matches::assert_matches;

    #[cfg_attr(feature = "track_caller", track_caller)]
    fn assert_analyze_ok(tree: &ExprTree, ctx: AnalyzeContext) {
        match analyze(tree, ctx) {
            Ok(_) => {}
            Err(e) => panic!("expected analyze to succeed, but got error: {:?}", e),
        }
    }

    #[cfg_attr(feature = "track_caller", track_caller)]
    fn assert_compile_error<T, F>(result: crate::Result<T>, check: F)
    where
        F: FnOnce(&CompileError) -> bool,
    {
        match result {
            Err(Error::CompileError(ref e)) if check(e) => {}
            other => panic!(
                "expected a matching CompileError, but got: {:?}",
                other.err()
            ),
        }
    }

    #[cfg_attr(feature = "track_caller", track_caller)]
    fn assert_invalid_backref(
        pattern: &str,
        explicit_capture_group_0: bool,
        expected_group: usize,
    ) {
        let tree = Expr::parse_tree(pattern).unwrap();
        assert_compile_error(
            analyze(
                &tree,
                AnalyzeContext {
                    explicit_capture_group_0,
                    ..Default::default()
                },
            ),
            |e| matches!(e, CompileError::InvalidBackref(g) if *g == expected_group),
        );
    }

    // #[test]
    // fn case_folding_safe() {
    //     let re = regex::Regex::new("(?i:ß)").unwrap();
    //     if re.is_match("SS") {
    //         assert!(!literal_const_size("ß", true));
    //     }

    //     // Another tricky example, Armenian ECH YIWN
    //     let re = regex::Regex::new("(?i:\\x{0587})").unwrap();
    //     if re.is_match("\u{0565}\u{0582}") {
    //         assert!(!literal_const_size("\u{0587}", true));
    //     }
    // }

    #[test]
    fn invalid_backref_zero() {
        assert_invalid_backref(r".\0", false, 0);
        assert_invalid_backref(r".\0", true, 0);
        assert_invalid_backref(r"(.)\0", false, 0);
        assert_invalid_backref(r"(.)\0", true, 0);
        assert_invalid_backref(r"(.)\0\1", false, 0);
    }

    #[test]
    fn invalid_backref_no_captures() {
        assert_invalid_backref(r"aa\1", false, 1);
        assert_invalid_backref(r"aaaa\2", false, 2);
    }

    #[test]
    fn invalid_backref_unreasonably_large_number() {
        // A group number that is a valid usize but far exceeds the number of groups in the
        // pattern. The resolver inserts it into the BitSet as-is; the analyzer catches it via
        // the total_groups bound check.
        assert_invalid_backref(r".\1999999999", false, 1999999999);
    }

    #[test]
    fn invalid_backref_with_captures() {
        assert_invalid_backref(r"a(a)\2", false, 2);
        assert_invalid_backref(r"a(a)\2\1", false, 2);
    }

    #[test]
    fn invalid_backref_with_captures_explict_capture_group_zero() {
        assert_invalid_backref(r"(a(b)\2)c", true, 2);
        assert_invalid_backref(r"(a(b)\1\2)c", true, 2);
        assert_invalid_backref(r"(a\1)b", true, 1);
        assert_invalid_backref(r"(a(b))\2", true, 2);
    }

    #[test]
    fn unresolved_subroutine_call_error_takes_precedence_over_invalid_backref() {
        // Regression test for issue where encountering an unresolved subroutine call would
        // cause the analyzer to stop visiting groups, leading to an incomplete group count.
        // This would then cause a misleading "Invalid back reference" error instead of
        // the correct error.
        let tree = Expr::parse_tree(r"(?<a>a)(?<b>b)\g<no_exist>(?<c>c)\k<a>\k<c>").unwrap();
        let result = analyze(&tree, AnalyzeContext::default());

        // Should get the unresolved subroutine call error, not an invalid backref error
        assert_compile_error(
            result,
            |e| matches!(e, CompileError::SubroutineCallTargetNotFound(s, _) if s.contains("no_exist")),
        );
    }

    #[test]
    fn allow_analysis_of_self_backref() {
        // even if it will never match, see issue 103
        assert!(!analyze(
            &Expr::parse_tree(r"(.\1)").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"((.\1))").unwrap(),
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            }
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"(([ab]+)\1b)").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
        // in the following scenario it can match
        assert!(!analyze(
            &Expr::parse_tree(r"(([ab]+?)(?(1)\1| )c)+").unwrap(),
            AnalyzeContext::default(),
        )
        .is_err());
    }

    #[test]
    fn allow_backref_even_when_capture_group_occurs_after_backref() {
        assert!(!analyze(
            &Expr::parse_tree(r"\1(.)").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"(\1(.))").unwrap(),
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            }
        )
        .is_err());
    }

    #[test]
    fn valid_backref_occurs_after_capture_group() {
        assert!(!analyze(
            &Expr::parse_tree(r"(.)\1").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"((.)\1)").unwrap(),
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            }
        )
        .is_err());

        assert!(!analyze(
            &Expr::parse_tree(r"((.)\2\2)\1").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"(.)\1(.)\2").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"(.)foo(.)\2").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"(.)(foo)(.)\3\2\1").unwrap(),
            AnalyzeContext::default(),
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"(.)(foo)(.)\3\1").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"(.)(foo)(.)\2\1").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
    }

    #[test]
    fn feature_not_yet_supported() {
        let tree = &Expr::parse_tree(r"(a)\k<1-0>").unwrap();
        assert_compile_error(analyze(tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::FeatureNotYetSupported(_))
        });
    }

    #[test]
    fn subroutine_call_undefined() {
        let tree = &Expr::parse_tree(r"\g<wrong_name>(?<different_name>a)").unwrap();
        assert_compile_error(analyze(tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::SubroutineCallTargetNotFound(_, _))
        });
    }

    #[test]
    fn subroutine_call_undefined_by_name_message() {
        let tree = &Expr::parse_tree(r"\g<wrong_name>(?<different_name>a)").unwrap();
        assert_compile_error(
            analyze(tree, AnalyzeContext::default()),
            |e| matches!(e, CompileError::SubroutineCallTargetNotFound(s, _) if s.contains("wrong_name")),
        );
    }

    #[test]
    fn subroutine_call_undefined_by_number() {
        use crate::parse::NamedGroups;
        use bit_set::BitSet;
        // ByNumber is always resolved by the parser so we must construct the ExprTree directly
        let tree = ExprTree {
            expr: Expr::AstNode(
                crate::AstNode::SubroutineCall(crate::CaptureGroupTarget::ByNumber(99)),
                0,
            ),
            backrefs: BitSet::new(),
            named_groups: NamedGroups::default(),
            numeric_capture_group_references: false,
            contains_subroutines: true,
            self_recursive: false,
            total_groups: 0,
            out_of_range_backref: None,
            numbered_groups_ignored: false,
        };
        assert_compile_error(
            analyze(&tree, AnalyzeContext::default()),
            |e| matches!(e, CompileError::SubroutineCallTargetNotFound(s, _) if s.contains("99")),
        );
    }

    #[test]
    fn subroutine_call_undefined_by_relative() {
        use crate::parse::NamedGroups;
        use bit_set::BitSet;
        // Relative can only fail to resolve on integer overflow (essentially never),
        // so we must construct the ExprTree directly
        let tree = ExprTree {
            expr: Expr::AstNode(
                crate::AstNode::SubroutineCall(crate::CaptureGroupTarget::Relative(-1)),
                0,
            ),
            backrefs: BitSet::new(),
            named_groups: NamedGroups::default(),
            numeric_capture_group_references: false,
            contains_subroutines: true,
            self_recursive: false,
            total_groups: 0,
            out_of_range_backref: None,
            numbered_groups_ignored: false,
        };
        assert_compile_error(
            analyze(&tree, AnalyzeContext::default()),
            |e| matches!(e, CompileError::SubroutineCallTargetNotFound(s, _) if s.contains("-1")),
        );
    }

    #[test]
    fn unresolved_ast_node_error() {
        use crate::parse::NamedGroups;
        use bit_set::BitSet;
        // Construct a tree with an unresolved Backref AstNode (not a SubroutineCall),
        // which should produce an UnresolvedAstNode error containing the node's debug representation
        let tree = ExprTree {
            expr: Expr::AstNode(
                crate::AstNode::Backref {
                    target: crate::CaptureGroupTarget::ByNumber(1),
                    casei: false,
                    relative_recursion_level: None,
                },
                0,
            ),
            backrefs: BitSet::new(),
            named_groups: NamedGroups::default(),
            numeric_capture_group_references: false,
            contains_subroutines: false,
            self_recursive: false,
            total_groups: 0,
            out_of_range_backref: None,
            numbered_groups_ignored: false,
        };
        assert_compile_error(
            analyze(&tree, AnalyzeContext::default()),
            |e| matches!(e, CompileError::UnresolvedAstNode(_, s) if s.contains("Backref")),
        );
    }

    #[test]
    fn numeric_capture_group_references_cannot_be_used_with_named_groups() {
        use crate::parse_flags::FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST;

        // Test case 1: Named group followed by numeric backref (no alternation)
        let tree = Expr::parse_tree_with_flags(
            r"(?<name>a)\1",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NamedBackrefOnly)
        });

        // Test case 2: Numeric backref followed by named group
        let tree = Expr::parse_tree_with_flags(
            r"(a)\1(?<name>b)",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NamedBackrefOnly)
        });

        // Test case 3: Alternation with numeric backref and named group in first branch
        let tree = Expr::parse_tree_with_flags(
            r"(?<name>a)\1|b",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NamedBackrefOnly)
        });

        // Test case 4: Alternation with named group in first branch, numeric backref in second
        let tree = Expr::parse_tree_with_flags(
            r"(?<name>a)|\1",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NamedBackrefOnly)
        });

        // Test case 5: Numbered group containing named group, with numeric backref
        let tree = Expr::parse_tree_with_flags(
            r"(a|(?<name>b))\1",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NamedBackrefOnly)
        });

        // Test case 6: Multiple branches with named groups and numeric backrefs
        let tree = Expr::parse_tree_with_flags(
            r"(?<x>a)|(?<y>b)|\1",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NamedBackrefOnly)
        });

        // Test case 7: Numeric subroutine call with named group
        let tree = Expr::parse_tree_with_flags(
            r"(?<foo>\w+)\g<1>",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NamedBackrefOnly)
        });

        // Test case 8: Named group with numeric subroutine call in alternation
        let tree = Expr::parse_tree_with_flags(
            r"(?<foo>a)|\g<1>",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NamedBackrefOnly)
        });

        // Positive cases - these should work

        // Only numeric backrefs and subroutine calls, no named groups
        let tree = Expr::parse_tree_with_flags(
            r"(a)(b+)\1\g<2>",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_analyze_ok(&tree, AnalyzeContext::default());

        // Only named groups and named backrefs and named subroutine calls
        let tree = Expr::parse_tree_with_flags(
            r"(?<name>a)\k<name>\g<name>",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_analyze_ok(&tree, AnalyzeContext::default());

        // Multiple named and numbered groups, no backrefs or subroutine calls
        let tree = Expr::parse_tree_with_flags(
            r"(?<a>a)|(?<b>b)(c)(d+)",
            FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST,
        )
        .unwrap();
        assert_analyze_ok(&tree, AnalyzeContext::default());
    }

    #[test]
    fn is_literal() {
        let tree = Expr::parse_tree("abc").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.is_literal_get_casei(), Some(false));
    }

    #[test]
    fn is_literal_casei() {
        let tree = Expr::parse_tree("(?i)abc").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.is_literal_get_casei(), Some(true));
    }

    #[test]
    fn is_literal_with_repeat() {
        let tree = Expr::parse_tree("abc*").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.is_literal_get_casei(), None);
    }

    #[test]
    fn anchored_for_starttext_assertions() {
        let tree = Expr::parse_tree(r"^(\w+)\1").unwrap();
        assert_eq!(can_compile_as_anchored(&tree.expr), true);

        let tree = Expr::parse_tree(r"^").unwrap();
        assert_eq!(can_compile_as_anchored(&tree.expr), true);
    }

    #[test]
    fn anchored_for_continue_from_prev_match_assertions() {
        let tree = Expr::parse_tree(r"\G(\w+)\1").unwrap();
        assert_eq!(can_compile_as_anchored(&tree.expr), true);
    }

    #[test]
    fn backref_inherits_group_size_info() {
        // Test that backrefs properly inherit min_size and const_size from referenced groups
        let tree = Expr::parse_tree(r"(abc)\1").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        // The concatenation should have min_size = 3 + 3 = 6 (group + backref)
        assert_eq!(info.min_size, 6);
        assert!(info.const_size);

        // Test with a variable-length group
        let tree = Expr::parse_tree(r"(a+)\1").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        // The group has min_size = 1, but const_size = false due to the +
        // So the total should be min_size = 2, const_size = false
        assert_eq!(info.min_size, 2);
        assert!(!info.const_size);

        // Test with optional group
        let tree = Expr::parse_tree(r"(a?)\1").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        // Both group and backref can be empty, so min_size = 0
        assert_eq!(info.min_size, 0);
        assert!(!info.const_size);
    }

    #[test]
    fn backref_forward_reference() {
        // Test forward references (backref before group definition)
        // These should use conservative defaults but still work
        let tree = Expr::parse_tree(r"\1(abc)").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        // Forward ref gets min_size=0, group gets min_size=3, total=3
        assert_eq!(info.min_size, 3);
        // Forward ref sets const_size=false, so overall is false
        assert!(!info.const_size);
    }

    #[test]
    fn backref_in_lookbehind() {
        assert!(!analyze(
            &Expr::parse_tree(r"(hello)(?<=\b\1)").unwrap(),
            AnalyzeContext::default(),
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"(..)(?<=\1\1)").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
        assert!(!analyze(
            &Expr::parse_tree(r"(abc)(?<=\1)def").unwrap(),
            AnalyzeContext::default()
        )
        .is_err());
    }

    #[test]
    fn backref_inside_recursed_group_not_supported() {
        let tree = Expr::parse_tree(r"(?<foo>a|\(\g<foo>\)\k<foo>?)").unwrap();
        assert_compile_error(
            analyze(&tree, AnalyzeContext::default()),
            |e| matches!(e, CompileError::FeatureNotYetSupported(s) if s.contains("Backreference") && s.contains("recursed")),
        );

        // Test with numbered groups
        let tree = Expr::parse_tree(r"(\g<1>\1)").unwrap();
        assert_compile_error(
            analyze(&tree, AnalyzeContext::default()),
            |e| matches!(e, CompileError::FeatureNotYetSupported(s) if s.contains("Backreference") && s.contains("recursed")),
        );

        // Another example with alternation inside recursed group
        let tree = Expr::parse_tree(r"(a|\g<1>b\1)").unwrap();
        assert_compile_error(
            analyze(&tree, AnalyzeContext::default()),
            |e| matches!(e, CompileError::FeatureNotYetSupported(s) if s.contains("Backreference") && s.contains("recursed")),
        );
    }

    #[test]
    fn backref_outside_recursed_group_is_allowed() {
        let tree = Expr::parse_tree(r"(?<foo>a|\(\g<foo>\))\k<foo>").unwrap();
        let result = analyze(&tree, AnalyzeContext::default());
        assert!(
            result.is_ok(),
            "Backref outside recursed group should pass analysis"
        );
    }

    #[test]
    fn not_anchored_for_startline_assertions() {
        let tree = Expr::parse_tree(r"(?m)^(\w+)\1").unwrap();
        assert_eq!(can_compile_as_anchored(&tree.expr), false);
    }

    #[test]
    fn start_line_analysis() {
        use crate::parse_flags::FLAG_ONIGURUMA_MODE;
        use crate::Assertion;

        let tree = Expr::parse_tree(r"(?m)^").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).expect("can analyze start line");
        assert_eq!(info.min_size, 0);
        assert!(info.const_size);
        assert!(!info.hard);
        assert_matches!(info.expr, Expr::Assertion(Assertion::StartLine { .. }));

        let tree = Expr::parse_tree_with_flags(r"(?m)^", FLAG_ONIGURUMA_MODE).unwrap();
        let info =
            analyze(&tree, AnalyzeContext::default()).expect("can analyze Oniguruma start line");
        assert_eq!(info.min_size, 0);
        assert!(info.hard);
        assert_matches!(
            info.expr,
            Expr::Assertion(Assertion::StartLineOniguruma { .. })
        );

        let tree = Expr::parse_tree_with_flags(r"(?m)(?=^)", FLAG_ONIGURUMA_MODE).unwrap();
        let info = analyze(&tree, AnalyzeContext::default())
            .expect("can analyze Oniguruma start line inside lookaround");
        assert_eq!(info.min_size, 0);
        assert!(info.hard);
        assert_matches!(
            info.children[0].expr,
            Expr::Assertion(Assertion::StartLineOniguruma { .. })
        );
        assert!(!info.children[0].hard);
    }

    #[test]
    fn min_pos_in_group_calculated_correctly_with_no_groups() {
        let tree = Expr::parse_tree(r"\G").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.min_size, 0);
        assert_eq!(info.min_pos_in_group, 0);
        assert!(info.const_size);

        let tree = Expr::parse_tree(r"\G(?=abc)\w+").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        // the lookahead itself has min size 0
        assert_eq!(info.children[1].min_size, 0);
        assert!(info.children[1].const_size);
        // the children of the lookahead have min_size 3 from the literal
        assert_eq!(info.children[1].children[0].min_size, 3);
        assert!(info.children[1].children[0].const_size);
        // after lookahead, the position is reset
        assert_eq!(info.children[2].min_pos_in_group, 0);
        assert_eq!(info.children[2].min_size, 1);
        assert_eq!(info.min_pos_in_group, 0);
        assert!(!info.const_size);

        let tree = Expr::parse_tree(r"(?:ab*|cd){2}(?=bar)\w").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        // the whole expression has min size 3 (a times 2 plus \w)
        assert_eq!(info.min_size, 3);
        // the min pos of the lookahead is 2
        assert_eq!(info.children[1].min_pos_in_group, 2);
        // after lookahead, the position is reset
        assert_eq!(info.children[2].min_pos_in_group, 2);
        assert_eq!(info.children[2].min_size, 1);
        assert!(!info.const_size);
    }

    #[test]
    fn backtracking_control_verb_is_hard_and_const_size() {
        let tree = Expr::parse_tree(r"(*FAIL)").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.min_size, 0);
        assert_eq!(info.min_pos_in_group, 0);
        assert!(info.const_size);
    }

    #[test]
    fn min_pos_in_group_calculated_correctly_with_capture_groups() {
        let tree = Expr::parse_tree(r"a(bc)d(e(f)g)").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.min_pos_in_group, 0);
        // before the capture begins, the min pos in group 0 is 1
        assert_eq!(info.children[1].min_pos_in_group, 1);
        // inside capture group 1, the min pos of the Concat inside the group is 0
        assert_matches!(info.children[1].children[0].expr, Expr::Concat(_));
        assert_eq!(info.children[1].children[0].min_pos_in_group, 0);
        assert!(info.children[1].children[0].const_size);
        // inside capture group 1, the min pos of the c inside the group is 1
        assert_matches!(info.children[1].children[0].children[1].expr, Expr::Literal { val, casei: false } if val == "c");
        assert_eq!(info.children[1].children[0].children[1].min_pos_in_group, 1);

        // prove we are looking at the position of the d after capture group 1
        assert_matches!(info.children[2].expr, Expr::Literal { val, casei: false } if val == "d");
        assert_eq!(info.children[2].min_pos_in_group, 3);
        assert_eq!(info.children[2].start_group(), 2);
        assert_eq!(info.children[2].min_size, 1);

        // prove we are looking at the position of the e in capture group 2
        assert_matches!(info.children[3].children[0].children[0].expr, Expr::Literal { val, casei: false } if val == "e");
        assert_eq!(info.children[3].children[0].children[0].min_pos_in_group, 0);
    }

    #[test]
    fn absent_repeater_is_hard_and_not_const_size() {
        let tree = Expr::parse_tree(r"(?~abc)").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.min_size, 0);
        assert!(!info.const_size);
        assert!(info.hard);
    }

    #[test]
    fn absent_expression_is_hard_and_not_const_size() {
        let tree = Expr::parse_tree(r"(?~|abc|\d+)").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        // min_size comes from exp part (\d+ has min_size 1)
        assert_eq!(info.min_size, 1);
        assert!(!info.const_size);
        assert!(info.hard);
    }

    #[test]
    fn range_clear_is_hard_and_const_size() {
        let tree = Expr::parse_tree(r"(?~|)").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.min_size, 0);
        assert!(info.const_size);
        assert!(info.hard);
    }

    #[test]
    fn left_recursive_subroutine_direct() {
        // Direct left recursion: group 1 calls itself at position 0
        let tree = Expr::parse_tree(r"(\g<1>a)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::LeftRecursiveSubroutineCall(_))
        });

        let tree = Expr::parse_tree(r"abc(\g<1>a)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::LeftRecursiveSubroutineCall(_))
        });
    }

    #[test]
    fn not_left_recursive_subroutine_after_group() {
        let tree = Expr::parse_tree(r"(a)\g<1>").unwrap();
        let result = analyze(&tree, AnalyzeContext::default());
        assert!(result.is_ok());

        let tree = Expr::parse_tree(r"(?<test>a)\g<test>").unwrap();
        let result = analyze(&tree, AnalyzeContext::default());
        assert!(result.is_ok());
    }

    #[test]
    fn left_recursive_subroutine_at_start() {
        // Left recursion at start of group: (\g<1>a)
        let tree = Expr::parse_tree(r"(\g<1>a)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::LeftRecursiveSubroutineCall(_))
        });

        let tree = Expr::parse_tree(r"(?<test>\g<test>a)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::LeftRecursiveSubroutineCall(_))
        });
    }

    #[test]
    fn left_recursive_subroutine_indirect() {
        // Indirect left recursion: non-nested subroutine calls to each other
        let tree = Expr::parse_tree(r"(\g<2>)(\g<1>)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::LeftRecursiveSubroutineCall(_))
        });

        let tree = Expr::parse_tree(r"(\g<2>)(\g<1>a)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::LeftRecursiveSubroutineCall(_))
        });
    }

    #[test]
    fn left_recursive_subroutine_with_alternation() {
        // Left recursion through alternation, depending which branch is taken it could be left-recursive
        let tree = Expr::parse_tree(r"(a|\g<1>)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::LeftRecursiveSubroutineCall(_))
        });
    }

    #[test]
    fn unbounded_recursive_after_char() {
        // Not left recursive because subroutine call is after a character was consumed,
        // but still never-ending because the recursive call is mandatory.
        let tree = Expr::parse_tree(r"(a\g<1>)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NeverEndingRecursion)
        });
    }

    #[test]
    fn bounded_recursive_after_char_is_allowed() {
        let tree = Expr::parse_tree(r"(a\g<1>?)").unwrap();
        let result = analyze(&tree, AnalyzeContext::default());
        assert!(result.is_ok());
    }

    #[test]
    fn not_left_recursive_zero_repetition() {
        // Not left recursive because subroutine call is unreachable
        let tree = Expr::parse_tree(r"(a?\g<1>){0}").unwrap();
        let result = analyze(&tree, AnalyzeContext::default());
        assert!(result.is_ok());
    }

    #[test]
    fn left_recursive_with_both_positions() {
        // Left recursive because \g<1> appears at position 0 in the group even though also at end at position 1
        let tree = Expr::parse_tree(r"(\g<1>a\g<1>)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::LeftRecursiveSubroutineCall(_))
        });
    }

    #[test]
    fn left_recursive_with_lookahead() {
        let tree = Expr::parse_tree(r"((?=a)\g<1>)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::LeftRecursiveSubroutineCall(_))
        });
    }

    #[test]
    fn self_recursive_group_zero() {
        // Self-recursive on group 0 after a character
        let tree = Expr::parse_tree(r"a\g<0>").unwrap();
        // Group 0 calls itself at position 1 (after 'a'), so this is NOT left recursive
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NeverEndingRecursion)
        });
    }

    #[test]
    fn not_left_recursive_forward_call() {
        // Forward subroutine call - not left recursive: \g<1>(a)
        let tree = Expr::parse_tree(r"\g<1>(a)").unwrap();
        let result = analyze(&tree, AnalyzeContext::default());
        // The call happens before the group is defined, but it's at position 0 of group 0 (implicit)
        // which calls group 1. Group 1 doesn't call anything, so no cycle.
        assert!(result.is_ok());
    }

    #[test]
    fn not_left_recursive_group_zero_explicit() {
        // Self-recursive on explicit group 0: (a\g<0>)
        let tree = Expr::parse_tree(r"(a\g<0>)").unwrap();
        assert_compile_error(
            analyze(
                &tree,
                AnalyzeContext {
                    explicit_capture_group_0: true,
                    ..Default::default()
                },
            ),
            |e| matches!(e, CompileError::NeverEndingRecursion),
        );
    }

    #[test]
    fn not_left_recursive_group_zero_subroutine_call_unreachable() {
        let tree = Expr::parse_tree(r"\g<0>{0}abc").unwrap();
        let result = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn left_recursive_group_zero_at_start() {
        // Self-recursive on explicit group 0 at start: (\g<0>a)
        let tree = Expr::parse_tree(r"(\g<0>a)").unwrap();
        let result = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        );
        // With explicit group 0, \g<0> at position 0 is left-recursive
        assert!(result.is_err());
    }

    #[test]
    fn three_way_indirect_recursion() {
        // Three-way indirect recursion where each recursive call is mandatory.
        let tree = Expr::parse_tree(r"(\g<2>)(\g<3>)(a\g<1>)").unwrap();
        // Group 1 -> Group 2 (at pos 0)
        // Group 2 -> Group 3 (at pos 0)
        // Group 3 -> Group 1 (at pos 1, after 'a')
        // This forms a cycle, but the call from group 3 to group 1 is at position 1
        // So it's not left-recursive
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::NeverEndingRecursion)
        });
    }

    #[test]
    fn three_way_left_recursive() {
        // Three-way left recursion
        let tree = Expr::parse_tree(r"(\g<2>)(\g<3>)(\g<1>)").unwrap();
        let result = analyze(&tree, AnalyzeContext::default());
        // Group 1 -> Group 2 (at pos 0)
        // Group 2 -> Group 3 (at pos 0)
        // Group 3 -> Group 1 (at pos 0)
        // This forms a left-recursive cycle
        assert!(result.is_err());

        let tree = Expr::parse_tree(r"(\g<2>a)(\g<3>b)(\g<1>c)").unwrap();
        let result = analyze(&tree, AnalyzeContext::default());
        assert!(result.is_err());
    }

    #[test]
    fn left_recursive_with_call_to_defined_group() {
        // Even though the call from Group 1 to Group 2 is inside {0} at root level,
        // Group 1's pattern can still be executed when called from Group 2
        // Group 1 contains a?\g<2> - calls group 2 (when executed)
        // Group 2 contains \g<1> - calls group 1 at position 0
        // This creates a cycle: Group 2 -> Group 1 (at pos 0) -> Group 2 (at pos 0)
        let tree = Expr::parse_tree(r"(a?\g<2>){0}(\g<1>)").unwrap();
        assert_compile_error(analyze(&tree, AnalyzeContext::default()), |e| {
            matches!(e, CompileError::LeftRecursiveSubroutineCall(_))
        });
    }

    #[test]
    fn no_left_recursion_complex_pattern() {
        // Group n (1): |\g<m>\g<n> - calls m then itself, but m has min_size > 0
        // Group m (2): a(b)\g<m> - calls itself after 'ab'
        let tree = Expr::parse_tree(r"(?<n>|\g<m>\g<n>)\z|\zEND (?<m>a(b)\g<m>)").unwrap();
        let result = analyze(&tree, AnalyzeContext::default());

        assert!(
            result.is_ok(),
            "Pattern should not be left-recursive because group m has min_size > 0"
        );
    }

    // Tests for forward-referenced subroutine calls
    // These verify that the analyzer correctly handles subroutine calls to groups
    // that are defined later in the pattern, and that group indexes are computed correctly.

    #[test]
    fn forward_subroutine_call_single_group() {
        // Forward reference: calls group 1 before it's defined
        let tree = Expr::parse_tree(r"\g<1>(.a.)").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();

        // The pattern should have 1 capture group
        assert_eq!(info.start_group(), 1);
        assert_eq!(info.end_group(), 2);

        // Verify the Info has the correct Expr nodes - a Concat with 2 children
        assert!(matches!(info.expr, Expr::Concat(_)));
        assert_eq!(info.children.len(), 2);

        // First child is the SubroutineCall
        assert!(matches!(info.children[0].expr, Expr::SubroutineCall(1)));
        // SubroutineCall should have group range 1..1 (no new groups)
        assert_eq!(info.children[0].start_group(), 1);
        assert_eq!(info.children[0].end_group(), 1);
        // SubroutineCall should get the min_size from the group
        assert_eq!(info.children[0].min_size, 3);
        assert!(info.children[0].const_size);

        // Second child is the group
        assert!(matches!(info.children[1].expr, Expr::Group(_)));
        assert_eq!(info.children[1].start_group(), 1);
        assert_eq!(info.children[1].end_group(), 2);
        // The group itself should also have min_size 3
        assert_eq!(info.children[1].min_size, 3);
        assert!(info.children[1].const_size);

        // Overall pattern should have min_size 6 (subroutine call + group)
        assert_eq!(info.min_size, 6);
        assert!(info.const_size);
    }

    #[test]
    fn forward_subroutine_call_with_multiple_groups() {
        // Forward reference with multiple groups defined after the subroutine call
        let tree = Expr::parse_tree(r"\g<2>(a)(bc+)").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();

        // The pattern should have 2 capture groups
        assert_eq!(info.start_group(), 1);
        assert_eq!(info.end_group(), 3);

        // Verify the Info Expr nodes - it's a Concat with 3 children
        assert!(matches!(info.expr, Expr::Concat(_)));
        assert_eq!(info.children.len(), 3);

        // SubroutineCall to group 2
        assert!(matches!(info.children[0].expr, Expr::SubroutineCall(2)));
        // The subroutine call itself is not in a capture group
        assert_eq!(info.children[0].start_group(), 1);
        assert_eq!(info.children[0].end_group(), 1);
        // SubroutineCall should get the min_size from the group
        assert_eq!(info.children[0].min_size, 2);
        assert!(!info.children[0].const_size);

        // First group (capture group 1)
        assert!(matches!(info.children[1].expr, Expr::Group(_)));
        assert_eq!(info.children[1].start_group(), 1);
        assert_eq!(info.children[1].end_group(), 2);
        assert_eq!(info.children[1].min_size, 1);
        assert!(info.children[1].const_size);

        // Second group (capture group 2)
        assert!(matches!(info.children[2].expr, Expr::Group(_)));
        assert_eq!(info.children[2].start_group(), 2);
        assert_eq!(info.children[2].end_group(), 3);
        assert_eq!(info.children[2].min_size, 2);
        assert!(!info.children[2].const_size);
    }

    #[test]
    fn forward_subroutine_call_with_nested_groups() {
        // Forward reference with multiple nested groups defined after the subroutine call
        let tree = Expr::parse_tree(r"(foo)\g<4>(a(b)?)(c(d))(?!e)").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();

        // The pattern should have 5 capture groups
        assert_eq!(info.start_group(), 1);
        assert_eq!(info.end_group(), 6);

        // Verify the Info Expr nodes - it's a Concat with 5 children
        assert!(matches!(info.expr, Expr::Concat(_)));
        assert_eq!(info.children.len(), 5);

        // SubroutineCall to group 4
        assert!(matches!(info.children[1].expr, Expr::SubroutineCall(4)));
        // The subroutine call itself is not in a capture group. but after group 1
        assert_eq!(info.children[1].start_group(), 2);
        assert_eq!(info.children[1].end_group(), 2);
        // SubroutineCall should get the min_size from the group
        assert_eq!(info.children[1].min_size, 2);
        assert!(info.children[1].const_size);

        // First group after the subroutine call (capture group 2)
        assert!(matches!(info.children[2].expr, Expr::Group(_)));
        assert_eq!(info.children[2].start_group(), 2);
        assert_eq!(info.children[2].end_group(), 4);
        assert_eq!(info.children[2].min_size, 1);
        assert!(!info.children[2].const_size);

        // Second group after the subroutine call (nested inside group 2)
        let group_info = &info.children[2].children[0].children[1].children[0];
        assert!(matches!(group_info.expr, Expr::Group(_)));
        assert_eq!(group_info.start_group(), 3);
        assert_eq!(group_info.end_group(), 4);
        assert_eq!(group_info.min_size, 1);
        assert!(group_info.const_size);

        // Third group after the subroutine call
        assert!(matches!(info.children[3].expr, Expr::Group(_)));
        assert_eq!(info.children[3].start_group(), 4);
        assert_eq!(info.children[3].end_group(), 6);
        assert_eq!(info.children[3].min_size, 2);
        assert!(info.children[3].const_size);

        // Fourth group after the subroutine call (nested inside group 4)
        let group_info = &info.children[3].children[0].children[1];
        assert!(matches!(group_info.expr, Expr::Group(_)));
        assert_eq!(group_info.start_group(), 5);
        assert_eq!(group_info.end_group(), 6);
        assert_eq!(group_info.min_size, 1);
        assert!(group_info.const_size);

        // Negative lookahead should start after group 5
        assert!(matches!(info.children[4].expr, Expr::LookAround(_, _)));
        assert_eq!(info.children[4].start_group(), 6);
        assert_eq!(info.children[4].end_group(), 6);
    }

    #[test]
    fn define_group_is_easy_zero_size() {
        // A DEFINE block should be analyzed as: min_size=0, const_size=true, hard=false
        // It matches nothing itself, so it doesn't need backtracking.
        let tree = Expr::parse_tree(r"(?(DEFINE)(?<word>\w+))").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();

        assert!(matches!(info.expr, Expr::DefineGroup { .. }));
        assert_eq!(info.min_size, 0);
        assert!(info.const_size);
        assert!(!info.hard);
    }

    #[test]
    fn define_group_assigns_group_numbers() {
        // Groups inside a DEFINE block should still be assigned group numbers,
        // so that subroutine calls can reference them.
        let tree = Expr::parse_tree(r"(?(DEFINE)(?<first>a)(?<second>b))").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();

        // The definitions child: a Concat of two groups (groups 1 and 2)
        assert_eq!(info.children[0].start_group(), 1);
        assert_eq!(info.children[0].end_group(), 3);

        // Group numbers are assigned in the usual order
        let tree = Expr::parse_tree(
            r"(abc)(?(DEFINE)(?<second>a)ignored: no group(?<third>b(?<fourth>c)))(?<fifth>d)",
        )
        .unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();

        assert_eq!(info.start_group(), 1);
        assert_eq!(info.end_group(), 6);
        assert_eq!(info.children[0].start_group(), 1);
        assert_eq!(info.children[1].children[0].start_group(), 2);
        assert_eq!(info.children[2].start_group(), 5);
        assert_eq!(info.children[2].end_group(), 6);
    }

    // Tests for max_size

    #[test]
    fn max_size_for_literal() {
        let tree = Expr::parse_tree("a").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, 1);
    }

    #[test]
    fn max_size_for_concat_of_literals() {
        let tree = Expr::parse_tree("abc").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, 3);
    }

    #[test]
    fn max_size_for_general_newline() {
        let tree = Expr::parse_tree(r"\R").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, 2);
    }

    #[test]
    fn max_size_for_greedy_star() {
        let tree = Expr::parse_tree("a*").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, usize::MAX);
    }

    #[test]
    fn max_size_for_greedy_plus() {
        let tree = Expr::parse_tree("a+").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, usize::MAX);
    }

    #[test]
    fn max_size_for_optional() {
        let tree = Expr::parse_tree("a?").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, 1);
    }

    #[test]
    fn max_size_for_bounded_repeat() {
        let tree = Expr::parse_tree("a{1,3}").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, 3);
    }

    #[test]
    fn max_size_for_exact_repeat() {
        let tree = Expr::parse_tree("a{3}").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, 3);
    }

    #[test]
    fn max_size_for_alternation() {
        let tree = Expr::parse_tree("a|bc").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, 2);
    }

    #[test]
    fn max_size_for_group() {
        let tree = Expr::parse_tree(r"(abc)").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, 3);
    }

    #[test]
    fn max_size_for_backref_inherits_group() {
        let tree = Expr::parse_tree(r"(ab)\1").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, 4);
    }

    #[test]
    fn max_size_for_concat_accumulates() {
        // ab*cd = concat(literal a, repeat b*, literal c, literal d)
        // max_size = 1 + MAX + 1 + 1 = MAX
        let tree = Expr::parse_tree("ab*cd").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, usize::MAX);
    }

    #[test]
    fn max_size_for_empty_assertion_is_zero() {
        let tree = Expr::parse_tree(r"\b").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_eq!(info.max_size, 0);
    }

    // Tests for leftmost_longest flag

    #[cfg(feature = "leftmost_longest")]
    #[test]
    fn leftmost_longest_rejects_non_greedy_star() {
        let tree = Expr::parse_tree(r"a*?").unwrap();
        let result = analyze(
            &tree,
            AnalyzeContext {
                leftmost_longest: true,
                ..Default::default()
            },
        );
        assert_compile_error(
            result,
            |e| matches!(e, CompileError::FeatureNotYetSupported(s) if s.contains("non-greedy")),
        );
    }

    #[cfg(feature = "leftmost_longest")]
    #[test]
    fn leftmost_longest_rejects_non_greedy_plus() {
        let tree = Expr::parse_tree(r"a+?").unwrap();
        let result = analyze(
            &tree,
            AnalyzeContext {
                leftmost_longest: true,
                ..Default::default()
            },
        );
        assert_compile_error(
            result,
            |e| matches!(e, CompileError::FeatureNotYetSupported(s) if s.contains("non-greedy")),
        );
    }

    #[cfg(feature = "leftmost_longest")]
    #[test]
    fn leftmost_longest_rejects_non_greedy_optional() {
        let tree = Expr::parse_tree(r"a??").unwrap();
        let result = analyze(
            &tree,
            AnalyzeContext {
                leftmost_longest: true,
                ..Default::default()
            },
        );
        assert_compile_error(
            result,
            |e| matches!(e, CompileError::FeatureNotYetSupported(s) if s.contains("non-greedy")),
        );
    }

    #[cfg(feature = "leftmost_longest")]
    #[test]
    fn leftmost_longest_rejects_non_greedy_bounded() {
        let tree = Expr::parse_tree(r"a{1,3}?").unwrap();
        let result = analyze(
            &tree,
            AnalyzeContext {
                leftmost_longest: true,
                ..Default::default()
            },
        );
        assert_compile_error(
            result,
            |e| matches!(e, CompileError::FeatureNotYetSupported(s) if s.contains("non-greedy")),
        );
    }

    #[cfg(feature = "leftmost_longest")]
    #[test]
    fn leftmost_longest_accepts_greedy_quantifiers() {
        assert_analyze_ok(
            &Expr::parse_tree(r"a*").unwrap(),
            AnalyzeContext {
                leftmost_longest: true,
                ..Default::default()
            },
        );
        assert_analyze_ok(
            &Expr::parse_tree(r"a+").unwrap(),
            AnalyzeContext {
                leftmost_longest: true,
                ..Default::default()
            },
        );
        assert_analyze_ok(
            &Expr::parse_tree(r"a?").unwrap(),
            AnalyzeContext {
                leftmost_longest: true,
                ..Default::default()
            },
        );
        assert_analyze_ok(
            &Expr::parse_tree(r"a{1,3}").unwrap(),
            AnalyzeContext {
                leftmost_longest: true,
                ..Default::default()
            },
        );
    }

    #[cfg(feature = "leftmost_longest")]
    #[test]
    fn leftmost_longest_promotes_non_const_to_hard() {
        // Without leftmost_longest, a* is not hard (can be delegated to regex-automata)
        let tree = Expr::parse_tree(r"a*").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert!(!info.hard);
        assert!(!info.const_size);

        // With leftmost_longest, a* becomes hard because it's not const_size
        let info = analyze(
            &tree,
            AnalyzeContext {
                leftmost_longest: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(info.hard);
        assert!(!info.const_size);
    }

    #[cfg(feature = "leftmost_longest")]
    #[test]
    fn leftmost_longest_keeps_const_size_easy() {
        let tree = Expr::parse_tree(r"abc").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                leftmost_longest: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!info.hard);
        assert!(info.const_size);
    }

    // Tests for EndText assertion with find_not_empty

    #[test]
    fn end_text_hard_with_find_not_empty() {
        let tree = Expr::parse_tree(r"$").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                find_not_empty: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(info.hard);
        assert!(info.const_size);
    }

    #[test]
    fn end_text_not_hard_without_find_not_empty() {
        let tree = Expr::parse_tree(r"$").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert!(!info.hard);
        assert!(info.const_size);
    }

    #[test]
    fn end_text_not_hard_in_lookaround_with_find_not_empty() {
        // Inside a lookaround, $ does not get the find_not_empty hard treatment
        let tree = Expr::parse_tree(r"(?=$)").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                find_not_empty: true,
                ..Default::default()
            },
        )
        .unwrap();
        // The lookahead's child is directly the EndText assertion
        let end_text_info = &info.children[0];
        assert!(!end_text_info.hard);
        assert!(end_text_info.const_size);
    }
}
