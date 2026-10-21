#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze::{analyze, AnalyzeContext};
    use crate::parse::ExprTree;
    use crate::vm::Insn::*;

    use matches::assert_matches;

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

    #[test]
    fn casei_literal_compiles_to_native_insn() {
        // A case-insensitive literal inside a hard pattern becomes a single
        // native LitCasei instruction, not a delegated engine.
        let prog = compile_prog_forced_hard("(?i)abc");
        assert_eq!(prog.len(), 2, "prog: {:?}", prog);
        assert_matches!(prog[0], LitCasei(_));
        assert_matches!(prog[1], End);
    }

    #[test]
    fn casei_alternation_branches_compile_to_native_insns() {
        // A hard element in one branch forces the alternation to compile
        // branch by branch on the VM (a fully-easy alternation would be
        // delegated whole). The literal branches must become native LitCasei
        // instructions instead of one delegated engine each.
        let prog = compile_prog_forced_hard("(?i)(abort|absent|z(?<=q)z)");
        let count = prog
            .iter()
            .filter(|insn| matches!(insn, LitCasei(_)))
            .count();
        assert!(
            count >= 2,
            "literal branches should compile to LitCasei: {:?}",
            prog
        );
        assert!(
            prog.iter().all(|insn| !matches!(insn, Insn::Delegate(_))),
            "no branch should need a forward delegate: {:?}",
            prog
        );
    }

    #[test]
    fn jumps_for_alternation() {
        let prog = compile_prog_forced_hard("a|b|c");

        assert_eq!(prog.len(), 8, "prog: {:?}", prog);
        assert_matches!(prog[0], Split(1, 3));
        assert_matches!(prog[1], Lit(ref l) if l == "a");
        assert_matches!(prog[2], Jmp(7));
        assert_matches!(prog[3], Split(4, 6));
        assert_matches!(prog[4], Lit(ref l) if l == "b");
        assert_matches!(prog[5], Jmp(7));
        assert_matches!(prog[6], Lit(ref l) if l == "c");
        assert_matches!(prog[7], End);
    }

    #[test]
    fn look_around_pattern_can_be_delegated() {
        let prog = compile_prog("(?=ab*)c");

        assert_eq!(prog.len(), 5, "prog: {:?}", prog);
        assert_matches!(prog[0], Save(0));
        assert_delegate_insn(&prog[1], "ab*", None);
        assert_matches!(prog[2], Restore(0));
        assert_matches!(prog[3], Lit(ref l) if l == "c");
        assert_matches!(prog[4], End);
    }

    #[test]
    fn easy_concat_can_delegate_end() {
        let prog = compile_prog("(?!x)(?:a|ab)x*");

        assert_eq!(prog.len(), 5, "prog: {:?}", prog);
        assert_matches!(prog[0], Split(1, 3));
        assert_matches!(prog[1], Lit(ref l) if l == "x");
        assert_matches!(prog[2], FailNegativeLookAround);
        assert_delegate_insn(&prog[3], "(?:a|ab)x*", None);
        assert_matches!(prog[4], End);
    }

    #[test]
    fn hard_concat_can_delegate_const_size_end() {
        let prog = compile_prog("(?:(?!x)(?:a|b)c)x*");

        assert_eq!(prog.len(), 6, "prog: {:?}", prog);
        assert_matches!(prog[0], Split(1, 3));
        assert_matches!(prog[1], Lit(ref l) if l == "x");
        assert_matches!(prog[2], FailNegativeLookAround);
        assert_delegate_insn(&prog[3], "(?:a|b)c", None);
        assert_delegate_insn(&prog[4], "x*", None);
        assert_matches!(prog[5], End);
    }

    #[test]
    fn hard_concat_can_not_delegate_variable_end() {
        let prog = compile_prog("(?:(?!x)(?:a|ab))x*");

        assert_eq!(prog.len(), 9, "prog: {:?}", prog);
        assert_matches!(prog[0], Split(1, 3));
        assert_matches!(prog[1], Lit(ref l) if l == "x");
        assert_matches!(prog[2], FailNegativeLookAround);
        assert_matches!(prog[3], Split(4, 6));
        assert_matches!(prog[4], Lit(ref l) if l == "a");
        assert_matches!(prog[5], Jmp(7));
        assert_matches!(prog[6], Lit(ref l) if l == "ab");
        assert_delegate_insn(&prog[7], "x*", None);
        assert_matches!(prog[8], End);
    }

    #[test]
    fn conditional_expression_can_be_compiled() {
        let prog = compile_prog(r"(?(ab)c|d)");

        assert_eq!(prog.len(), 8, "prog: {:?}", prog);

        assert_matches!(prog[0], BeginAtomic);
        assert_matches!(prog[1], Split(2, 6));
        assert_matches!(prog[2], Lit(ref l) if l == "ab");
        assert_matches!(prog[3], EndAtomic);
        assert_matches!(prog[4], Lit(ref l) if l == "c");
        assert_matches!(prog[5], Jmp(7));
        assert_matches!(prog[6], Lit(ref l) if l == "d");
        assert_matches!(prog[7], End);
    }

    #[test]
    fn lazy_any_can_be_compiled_explicit_capture_group_zero() {
        let prog = compile_prog(r"\O*?((?!a))");

        assert_eq!(prog.len(), 9, "prog: {:?}", prog);

        assert_matches!(prog[0], Split(3, 1));
        assert_matches!(prog[1], Any);
        assert_matches!(prog[2], Jmp(0));
        assert_matches!(prog[3], SaveCaptureGroupStart(0));
        assert_matches!(prog[4], Split(5, 7));
        assert_matches!(prog[5], Lit(ref l) if l == "a");
        assert_matches!(prog[6], FailNegativeLookAround);
        assert_matches!(prog[7], Save(1));
        assert_matches!(prog[8], End);
    }

    #[test]
    fn backtracking_control_verb_fail_can_be_compiled() {
        let prog = compile_prog(r"(*FAIL)");

        assert_eq!(prog.len(), 2, "prog: {:?}", prog);

        assert_matches!(prog[0], Fail);
        assert_matches!(prog[1], End);
    }

    #[test]
    fn other_backtracking_control_verbs_error() {
        let tree = Expr::parse_tree(r"(*ACCEPT)").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
        );

        let tree = Expr::parse_tree(r"(*COMMIT)").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
        );

        let tree = Expr::parse_tree(r"(*SKIP)").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
        );

        let tree = Expr::parse_tree(r"(*PRUNE)").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
        );
    }

    #[test]
    fn backref_exists_condition_with_recursion_level_not_yet_supported() {
        for pattern in &[
            r"(a)(?(1+0)b|c)d",
            r"(?<n>a)(?(<n+0>)b|c)d",
            r"(?<n>a)(?('n+0')b|c)d",
        ] {
            let tree = Expr::parse_tree(pattern).unwrap();
            let info = analyze(
                &tree,
                AnalyzeContext {
                    explicit_capture_group_0: false,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_compile_error(
                compile(
                    &info,
                    CompileOptions {
                        anchored: true,
                        contains_subroutines: tree.contains_subroutines,
                        ..CompileOptions::default()
                    },
                ),
                |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
            );
        }
    }

    #[test]
    #[cfg(not(feature = "variable-lookbehinds"))]
    fn variable_lookbehind_requires_feature() {
        // Without the feature flag, variable-length lookbehinds should error
        let tree = Expr::parse_tree(r"(?<=ab+)x").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::VariableLookBehindRequiresFeature),
        );

        let tree = Expr::parse_tree(r"(?<=\bab+)x").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::VariableLookBehindRequiresFeature),
        );
    }

    #[test]
    #[cfg(feature = "variable-lookbehinds")]
    fn variable_lookbehind_with_required_feature_no_captures_easy() {
        let prog = compile_prog(r"(?<=ab+)x");

        assert_eq!(prog.len(), 5, "prog: {:?}", prog);

        assert_matches!(prog[0], Save(0));
        assert_matches!(&prog[1], BackwardsDelegate(ReverseBackwardsDelegate { pattern, dfa: _, cache_pool: _, capture_group_extraction_inner: None, capture_groups: None }) if pattern == "ab+");
        assert_matches!(prog[2], Restore(0));
        assert_matches!(prog[3], Lit(ref l) if l == "x");
        assert_matches!(prog[4], End);
    }

    #[test]
    #[cfg(feature = "variable-lookbehinds")]
    fn variable_lookbehind_with_required_feature_no_captures_hard_const_size_zero_length() {
        let prog = compile_prog(r"(?<=\bab+)x");

        assert_eq!(prog.len(), 6, "prog: {:?}", prog);

        assert_matches!(prog[0], Save(0));
        assert_matches!(&prog[1], BackwardsDelegate(ReverseBackwardsDelegate { pattern, dfa: _, cache_pool: _, capture_group_extraction_inner: None, capture_groups: None }) if pattern == "ab+");
        assert_matches!(prog[2], Insn::Assertion(crate::Assertion::WordBoundary));
        assert_matches!(prog[3], Restore(0));
        assert_matches!(prog[4], Lit(ref l) if l == "x");
        assert_matches!(prog[5], End);
    }

    #[test]
    #[cfg(feature = "variable-lookbehinds")]
    fn variable_lookbehind_with_required_feature_no_captures_hard_const_size_non_zero_length() {
        let prog = compile_prog(r"((.)b+(?<=\1\1b+)x)");

        assert_eq!(prog.len(), 16, "prog: {:?}", prog);

        assert_matches!(prog[0], SaveCaptureGroupStart(0));
        assert_matches!(prog[1], SaveCaptureGroupStart(1));
        assert_matches!(prog[2], AnyNoNL);
        assert_matches!(prog[3], Save(3));
        assert_matches!(prog[4], Lit(ref l) if l == "b");
        assert_matches!(prog[5], Split(4, 6));
        assert_matches!(prog[6], Save(4));
        assert_matches!(&prog[7], BackwardsDelegate(ReverseBackwardsDelegate { pattern, dfa: _, cache_pool: _, capture_group_extraction_inner: None, capture_groups: None }) if pattern == "b+");
        assert_matches!(prog[8], GoBack(1));
        assert_matches!(
            prog[9],
            Backref {
                slot: 2,
                casei: false,
                unicode: true,
            }
        );
        assert_matches!(prog[10], GoBack(2));
        assert_matches!(
            prog[11],
            Backref {
                slot: 2,
                casei: false,
                unicode: true,
            }
        );
        assert_matches!(prog[12], Restore(4));
        assert_matches!(prog[13], Lit(ref l) if l == "x");
        assert_matches!(prog[14], Save(1));
        assert_matches!(prog[15], End);
    }

    #[test]
    #[cfg(feature = "variable-lookbehinds")]
    fn variable_lookbehind_with_required_feature_captures() {
        let prog = compile_prog(r"(?<=a(b+))x");

        assert_eq!(prog.len(), 5, "prog: {:?}", prog);

        assert_matches!(prog[0], Save(2));
        assert_matches!(&prog[1], BackwardsDelegate(ReverseBackwardsDelegate { pattern, dfa: _, cache_pool: _, capture_group_extraction_inner: ref inner, capture_groups: Some(CaptureGroupRange(0, 1)) }) if pattern == "a(b+)" && inner.is_some());
        assert_matches!(prog[2], Restore(2));
        assert_matches!(prog[3], Lit(ref l) if l == "x");
        assert_matches!(prog[4], End);
    }

    #[test]
    #[cfg(feature = "variable-lookbehinds")]
    fn variable_lookbehind_with_required_feature_backref_captures() {
        // currently hard variable lookbehinds are unsupported.
        // the backref to a capture group inside the variable lookbehind makes the capture group hard
        let tree = Expr::parse_tree(r"(?<=a(b+))\1").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
        );
    }

    #[test]
    fn absent_repeater_with_easy_inner_compiles() {
        let prog = compile_prog(r"((?~abc))");

        assert_eq!(prog.len(), 4, "prog: {:?}", prog);
        assert_matches!(prog[0], SaveCaptureGroupStart(0));
        assert_absent_repeater_insn(&prog[1], "abc", None);
        assert_matches!(prog[2], Save(1));
        assert_matches!(prog[3], End);
    }

    #[test]
    fn absent_repeater_with_hard_inner_compiles() {
        // A hard absent repeater (?~\1) expands to (?((?!\1))\O|)*
        // Use explicit_capture_group_0=false so that (\w) = group 1 and \1 refers to it
        let prog = compile_prog_no_explicit_group0(r"(\w)(?~\1)");

        assert_eq!(prog.len(), 20, "prog: {:?}", prog);

        assert_matches!(prog[0], SplitUnanchored(3, 1));
        assert_matches!(prog[1], Any);
        assert_matches!(prog[2], Jmp(0));
        assert_matches!(prog[3], Save(0));
        assert_matches!(prog[4], SaveCaptureGroupStart(1));
        assert_char_class_insn(&prog[5], r"\w");
        assert_matches!(prog[6], Save(3));
        assert_matches!(prog[7], Save0(4));
        assert_matches!(
            prog[8],
            RepeatEpsilonGr {
                lo: 0,
                next: 18,
                repeat: 4,
                check: 5
            }
        );
        assert_matches!(prog[9], BeginAtomic);
        assert_matches!(prog[10], Split(11, 17));
        assert_matches!(prog[11], Split(12, 14));
        assert_matches!(
            prog[12],
            Backref {
                slot: 2,
                casei: false,
                unicode: true,
            }
        );
        assert_matches!(prog[13], FailNegativeLookAround);
        assert_matches!(prog[14], EndAtomic);
        assert_matches!(prog[15], Any);
        assert_matches!(prog[16], Jmp(17));
        assert_matches!(prog[17], Jmp(8));
        assert_matches!(prog[18], Save(1));
        assert_matches!(prog[19], End);
    }

    #[test]
    fn absent_repeater_nested_absent_error() {
        // Nested absent operators are not yet supported (direct child)
        let tree = Expr::parse_tree(r"(?~(?~abc))").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
        );

        // Nested absent operators are not yet supported (indirect descendant)
        let tree = Expr::parse_tree(r"(?~a(?<=b(?~c)))").unwrap();
        let info = analyze(&tree, AnalyzeContext::default()).unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
        );
    }

    #[test]
    fn absent_operators_error() {
        // Test that absent expression returns feature not supported
        let tree = Expr::parse_tree(r"(?~|abc|\d*)").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
        );

        // Test that absent stopper returns feature not supported
        let tree = Expr::parse_tree(r"(?~|abc)").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
        );

        // Test that range clear returns feature not supported
        let tree = Expr::parse_tree(r"(?~|)").unwrap();
        let info = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_compile_error(
            compile(
                &info,
                CompileOptions {
                    anchored: true,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            ),
            |e| matches!(e, CompileError::FeatureNotYetSupported(_)),
        );
    }

    #[test]
    fn single_delegate_with_capture_groups_can_be_compiled() {
        let prog = compile_prog(r"(.(b)([^a]+))c");

        assert_eq!(prog.len(), 2, "prog: {:?}", prog);
        assert_delegate_insn(&prog[0], "(.(b)([^a]+))c", Some(CaptureGroupRange(0, 3)));
        assert_matches!(prog[1], End);
    }

    #[test]
    fn delegated_capture_groups_can_be_compiled() {
        let prog = compile_prog(r"(.(b)([^a]+)(?!c)(\w))");

        assert_eq!(prog.len(), 12, "prog: {:?}", prog);

        assert_matches!(prog[0], SaveCaptureGroupStart(0));
        assert_delegate_insn(&prog[1], ".(b)", Some(CaptureGroupRange(1, 2)));
        assert_matches!(prog[2], SaveCaptureGroupStart(2));
        assert_char_class_insn(&prog[3], "[^a]");
        assert_matches!(prog[4], Split(3, 5));
        assert_matches!(prog[5], Save(5));
        assert_matches!(prog[6], Split(7, 9));
        assert_matches!(prog[7], Lit(ref l) if l == "c");
        assert_matches!(prog[8], FailNegativeLookAround);
        assert_delegate_insn(&prog[9], r"(\w)", Some(CaptureGroupRange(3, 4)));
        assert_matches!(prog[10], Save(1));
        assert_matches!(prog[11], End);
    }

    #[test]
    fn subroutine_call_can_be_compiled() {
        let prog = compile_prog(r"((.)\g<1>)");

        assert_eq!(prog.len(), 7, "prog: {:?}", prog);

        assert_matches!(prog[0], SaveCaptureGroupStart(0));
        assert_delegate_insn(&prog[1], "(.)", Some(CaptureGroupRange(1, 2)));
        assert_matches!(prog[2], SaveCaptureGroupStart(1));
        assert_matches!(prog[3], AnyNoNL);
        assert_matches!(prog[4], Save(3));
        assert_matches!(prog[5], Save(1));
        assert_matches!(prog[6], End);
    }

    #[test]
    fn forward_reference_subroutine_call_can_be_compiled() {
        let prog = compile_prog(r"(\g<1>(.))");

        assert_eq!(prog.len(), 7, "prog: {:?}", prog);

        assert_matches!(prog[0], SaveCaptureGroupStart(0));
        assert_matches!(prog[1], SaveCaptureGroupStart(1));
        assert_matches!(prog[2], AnyNoNL);
        assert_matches!(prog[3], Save(3));
        assert_delegate_insn(&prog[4], "(.)", Some(CaptureGroupRange(1, 2)));
        assert_matches!(prog[5], Save(1));
        assert_matches!(prog[6], End);
    }

    #[test]
    fn define_group_is_a_no_op() {
        // A DEFINE block is not hard but produces an empty delegate regex,
        // so compilation skips it entirely — only the End instruction remains.
        let prog = compile_prog(r"(?(DEFINE)(?<word>\w+))");

        assert_eq!(prog.len(), 1, "prog: {:?}", prog);
        assert_matches!(prog[0], End);
    }

    #[test]
    fn continue_from_prev_match_inside_variable_lookbehind_alt_can_be_compiled() {
        let prog = compile_prog(r"(?<=\G|\s)\d");

        assert_eq!(prog.len(), 12, "prog: {:?}", prog);

        assert_matches!(prog[0], Split(1, 6));
        assert_matches!(prog[1], Save(0));
        assert_matches!(prog[2], GoBack(0));
        assert_matches!(prog[3], ContinueFromPreviousMatchEnd { at_start: false });
        assert_matches!(prog[4], Restore(0));
        assert_matches!(prog[5], Jmp(10));
        assert_matches!(prog[6], Save(1));
        assert_matches!(prog[7], GoBack(1));
        assert_char_class_insn(&prog[8], r"\s");
        assert_matches!(prog[9], Restore(1));
        assert_char_class_insn(&prog[10], r"\d");
        assert_matches!(prog[11], End);
    }

    #[test]
    fn continue_from_prev_match_inside_alt_can_be_compiled() {
        let prog = compile_prog(r"(?:\G|\s)\d");

        assert_eq!(prog.len(), 6, "prog: {:?}", prog);

        assert_matches!(prog[0], Split(1, 3));
        assert_matches!(prog[1], ContinueFromPreviousMatchEnd { at_start: false });
        assert_matches!(prog[2], Jmp(4));
        assert_char_class_insn(&prog[3], r"\s");
        assert_char_class_insn(&prog[4], r"\d");
        assert_matches!(prog[5], End);
    }

    #[test]
    fn continue_from_prev_match_first_instruction_can_be_compiled() {
        let prog = compile_prog(r"\G\s\d");

        assert_eq!(prog.len(), 3, "prog: {:?}", prog);

        assert_matches!(prog[0], ContinueFromPreviousMatchEnd { at_start: true });
        assert_delegate_insn(&prog[1], r"\s\d", None);
        assert_matches!(prog[2], End);

        let prog = compile_prog(r"^\G\s\d");

        assert_eq!(prog.len(), 4, "prog: {:?}", prog);

        assert_matches!(prog[1], ContinueFromPreviousMatchEnd { at_start: true });
        assert_delegate_insn(&prog[2], r"\s\d", None);
        assert_matches!(prog[3], End);
    }

    #[test]
    fn continue_from_prev_match_not_first_instruction_can_be_compiled() {
        let prog = compile_prog(r"\w\G\s\d");

        assert_eq!(prog.len(), 4, "prog: {:?}", prog);

        assert_char_class_insn(&prog[0], r"\w");
        assert_matches!(prog[1], ContinueFromPreviousMatchEnd { at_start: false });
        assert_delegate_insn(&prog[2], r"\s\d", None);
        assert_matches!(prog[3], End);
    }

    #[test]
    fn continue_from_prev_match_inside_conditional_can_be_compiled() {
        let prog = compile_prog(r"(?(\G)\s\d|$)");

        assert_eq!(prog.len(), 8, "prog: {:?}", prog);

        assert_matches!(prog[0], BeginAtomic);
        assert_matches!(prog[1], Split(2, 6));
        assert_matches!(prog[2], ContinueFromPreviousMatchEnd { at_start: false });
        assert_matches!(prog[3], EndAtomic);
        assert_delegate_insn(&prog[4], r"\s\d", None);
        assert_matches!(prog[5], Jmp(7));
        assert_delegate_insn(&prog[6], r"$", None);
        assert_matches!(prog[7], End);
    }

    fn compile_prog(re: &str) -> Vec<Insn> {
        compile_prog_with(
            re,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
            |info, tree| {
                compile(
                    info,
                    CompileOptions {
                        anchored: true,
                        contains_subroutines: tree.contains_subroutines,
                        ..CompileOptions::default()
                    },
                )
                .unwrap()
                .body
            },
        )
    }

    fn compile_prog_no_explicit_group0(re: &str) -> Vec<Insn> {
        compile_prog_with(re, AnalyzeContext::default(), |info, tree| {
            compile(
                info,
                CompileOptions {
                    anchored: false,
                    contains_subroutines: tree.contains_subroutines,
                    ..CompileOptions::default()
                },
            )
            .unwrap()
            .body
        })
    }

    fn compile_prog_forced_hard(re: &str) -> Vec<Insn> {
        compile_prog_with(
            re,
            AnalyzeContext {
                explicit_capture_group_0: true,
                ..Default::default()
            },
            |info, tree| {
                info.hard = true;
                compile(
                    info,
                    CompileOptions {
                        anchored: true,
                        contains_subroutines: tree.contains_subroutines,
                        ..CompileOptions::default()
                    },
                )
                .unwrap()
                .body
            },
        )
    }

    fn compile_prog_with<F>(re: &str, analyze_context: AnalyzeContext, compile_fn: F) -> Vec<Insn>
    where
        F: FnOnce(&mut Info<'_>, &ExprTree) -> Vec<Insn>,
    {
        let tree = Expr::parse_tree(re).unwrap();
        let mut info = analyze(&tree, analyze_context).unwrap();
        compile_fn(&mut info, &tree)
    }

    fn assert_delegate_insn(insn: &Insn, re: &str, captures: Option<CaptureGroupRange>) {
        match insn {
            Insn::Delegate(delegate) => assert_delegate(delegate, re, captures),
            _ => {
                panic!("Expected Insn::Delegate but was {:#?}", insn);
            }
        }
    }

    /// Assert that `insn` is a native `CharClass` matching the single character
    /// class `re` (compiled with default options, matching `compile_prog`).
    fn assert_char_class_insn(insn: &Insn, re: &str) {
        let expected = try_char_class_matcher(re, &CompileOptions::default())
            .unwrap_or_else(|| panic!("test re {:?} is not a single char class", re));
        match insn {
            Insn::CharClass(matcher) => assert_eq!(
                matcher, &expected,
                "char class mismatch for {:?}: {:#?}",
                re, matcher
            ),
            _ => panic!("Expected Insn::CharClass but was {:#?}", insn),
        }
    }

    fn assert_absent_repeater_insn(insn: &Insn, re: &str, captures: Option<CaptureGroupRange>) {
        match insn {
            Insn::AbsentRepeater(delegate) => assert_delegate(delegate, re, captures),
            _ => {
                panic!("Expected Insn::AbsentRepeater but was {:#?}", insn);
            }
        }
    }

    fn assert_delegate(
        delegate: &crate::vm::Delegate,
        re: &str,
        captures: Option<CaptureGroupRange>,
    ) {
        assert_eq!(delegate.pattern, re);
        assert_eq!(captures, delegate.capture_groups);
    }
}
