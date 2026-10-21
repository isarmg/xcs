#[cfg(test)]
mod tests {
    use super::optimize;
    use super::vec;
    use super::Arc;
    use super::Box;
    use crate::parse::make_literal;
    use crate::Expr;
    use alloc::string::String;

    fn optimized_pattern(pattern: &str) -> String {
        optimized_pattern_with_flags(pattern, crate::parse_flags::FLAG_UNICODE)
    }

    fn optimized_pattern_with_flags(pattern: &str, flags: u32) -> String {
        let mut tree = Expr::parse_tree_with_flags(pattern, flags).unwrap();
        optimize(&mut tree);
        let mut s = String::new();
        tree.expr.to_str(&mut s, 0);
        s
    }

    fn oniguruma_flags() -> u32 {
        crate::parse_flags::FLAG_ONIGURUMA_MODE | crate::parse_flags::FLAG_UNICODE
    }

    #[test]
    fn trailing_positive_lookahead_optimized() {
        let mut tree = Expr::parse_tree("a(?=b)").unwrap();
        let requires_capture_group_fixup = optimize(&mut tree);
        assert_eq!(requires_capture_group_fixup, true);
        let mut s = String::new();
        tree.expr.to_str(&mut s, 0);
        assert_eq!(s, "(a)b");
    }

    #[test]
    fn standalone_positive_lookahead_optimized() {
        let mut tree = Expr::parse_tree("(?=b)").unwrap();
        let requires_capture_group_fixup = optimize(&mut tree);
        assert_eq!(requires_capture_group_fixup, true);
        let mut s = String::new();
        tree.expr.to_str(&mut s, 0);
        assert_eq!(s, "()b");
    }

    #[test]
    fn trailing_positive_lookahead_with_alternative_optimized() {
        let mut tree = Expr::parse_tree("a(?=b|c)").unwrap();
        let requires_capture_group_fixup = optimize(&mut tree);
        assert_eq!(requires_capture_group_fixup, true);
        let mut s = String::new();
        tree.expr.to_str(&mut s, 0);
        assert_eq!(s, "(a)(?:b|c)");
    }

    #[test]
    fn trailing_positive_lookahead_moved_even_if_not_easy() {
        let mut tree = Expr::parse_tree(r"(a)\1(?=c)").unwrap();
        let requires_capture_group_fixup = optimize(&mut tree);
        assert_eq!(requires_capture_group_fixup, true);
        assert_eq!(
            tree.expr,
            Expr::Concat(vec![
                Expr::Group(Arc::new(Expr::Concat(vec![
                    Expr::Group(Arc::new(make_literal("a"))),
                    Expr::Backref {
                        group: 1,
                        casei: false
                    }
                ]))),
                make_literal("c"),
            ])
        );
    }

    #[test]
    fn trailing_positive_lookahead_left_alone_when_self_recursive() {
        let tree = Expr::parse_tree(r"ab?\g<0>?(?=a|$)").unwrap();
        let mut optimized_tree = tree.clone();
        let requires_capture_group_fixup = optimize(&mut optimized_tree);
        assert_eq!(requires_capture_group_fixup, false);
        assert_eq!(&optimized_tree.expr, &tree.expr);
    }

    #[test]
    fn trailing_negative_lookahead_left_alone() {
        let tree = Expr::parse_tree(r"a(?!b)").unwrap();
        let mut optimized_tree = tree.clone();
        let requires_capture_group_fixup = optimize(&mut optimized_tree);
        assert_eq!(requires_capture_group_fixup, false);
        assert_eq!(&optimized_tree.expr, &tree.expr);
    }

    #[test]
    fn trailing_positive_lookbehind_left_alone() {
        let tree = Expr::parse_tree(r"(?<=b)").unwrap();
        let mut optimized_tree = tree.clone();
        let requires_capture_group_fixup = optimize(&mut optimized_tree);
        assert_eq!(requires_capture_group_fixup, false);
        assert_eq!(&optimized_tree.expr, &tree.expr);
    }

    #[test]
    fn non_trailing_positive_lookahead_left_alone() {
        let tree = Expr::parse_tree(r"a(?=(b))\1").unwrap();
        let mut optimized_tree = tree.clone();
        let requires_capture_group_fixup = optimize(&mut optimized_tree);
        assert_eq!(requires_capture_group_fixup, false);
        assert_eq!(&optimized_tree.expr, &tree.expr);

        let tree = Expr::parse_tree(r"(?=(b))\1").unwrap();
        let mut optimized_tree = tree.clone();
        let requires_capture_group_fixup = optimize(&mut optimized_tree);
        assert_eq!(requires_capture_group_fixup, false);
        assert_eq!(&optimized_tree.expr, &tree.expr);
    }

    #[test]
    fn keepout_optimized() {
        let mut tree = Expr::parse_tree(r"ab\Kcd").unwrap();
        let requires_capture_group_fixup = optimize(&mut tree);
        assert_eq!(requires_capture_group_fixup, true);
        let mut s = String::new();
        tree.expr.to_str(&mut s, 0);
        assert_eq!(s, "ab(cd)");
    }

    #[test]
    fn keepout_at_start_of_concat_optimized() {
        let mut tree = Expr::parse_tree(r"\Kcd").unwrap();
        let requires_capture_group_fixup = optimize(&mut tree);
        assert_eq!(requires_capture_group_fixup, true);
        let mut s = String::new();
        tree.expr.to_str(&mut s, 0);
        assert_eq!(s, "(cd)");
    }

    #[test]
    fn keepout_with_trailing_lookahead_optimized_into_single_group() {
        // \K and trailing positive lookahead should be merged into one capture group,
        // not two separate group-0 wrappers.
        let mut tree = Expr::parse_tree(r#"ab\Kcd(?=e)"#).unwrap();
        let requires_capture_group_fixup = optimize(&mut tree);
        assert_eq!(requires_capture_group_fixup, true);
        let mut s = String::new();
        tree.expr.to_str(&mut s, 0);
        assert_eq!(s, "ab(cd)e");
    }

    #[test]
    fn keepout_left_alone_when_self_recursive() {
        let tree = Expr::parse_tree(r"ab?\g<0>?\Kc").unwrap();
        let mut optimized_tree = tree.clone();
        let requires_capture_group_fixup = optimize(&mut optimized_tree);
        assert_eq!(requires_capture_group_fixup, false);
        assert_eq!(&optimized_tree.expr, &tree.expr);
    }

    #[test]
    fn keepout_inside_group_left_alone() {
        // \K inside a capturing group (not the root concat) should not be optimized
        let tree = Expr::parse_tree(r"c(a\Kb)d").unwrap();
        let mut optimized_tree = tree.clone();
        let requires_capture_group_fixup = optimize(&mut optimized_tree);
        assert_eq!(requires_capture_group_fixup, false);
        assert_eq!(&optimized_tree.expr, &tree.expr);
    }

    #[test]
    fn keepout_with_prefix_capture_group_left_alone() {
        // When there's a capture group before \K, applying the optimization would make
        // that group become the first group (group 1) instead of our new wrapper, so
        // we must leave the pattern unchanged.
        let tree = Expr::parse_tree(r"(\w+)\K\s*(\w+)").unwrap();
        let mut optimized_tree = tree.clone();
        let requires_capture_group_fixup = optimize(&mut optimized_tree);
        assert_eq!(requires_capture_group_fixup, false);
        assert_eq!(&optimized_tree.expr, &tree.expr);
    }

    #[test]
    fn nested_plus_plus_simplified() {
        assert_eq!(optimized_pattern(r"(x+){1,}"), "(x+)");
    }

    #[test]
    fn nested_star_plus_simplified() {
        assert_eq!(optimized_pattern(r"(x*){1,}"), "(x*)");
    }

    #[test]
    fn nested_optional_plus_simplified() {
        assert_eq!(optimized_pattern(r"(x?){1,}"), "(x*)");
    }

    #[test]
    fn nested_optional_optional_not_simplified() {
        // (x?)? — outer is Optional (lo=0), so the capture group can be
        // skipped entirely (→ None). Simplifying to (x?) would always enter
        // the group (→ Some("")), changing capture semantics.
        assert_eq!(optimized_pattern(r"(x?){0,1}"), "(x?)?");
    }

    #[test]
    fn nested_optional_star_not_simplified() {
        // Outer (x*){?} has lo=0, so the capture group can be
        // skipped (→ None). Simplifying would always enter the group.
        assert_eq!(optimized_pattern(r"(x*){0,1}"), "(x*)?");
    }

    #[test]
    fn nested_repeats_in_children_simplified() {
        // (x+){1,} is simplified (outer lo=1, group always participates).
        // (y*){0,} has outer lo=0, so the group can be skipped → None.
        // With no backreferences and a self-absorbing group content (y*),
        // the outer {0,} (*) is downgraded to ? since (y*)* = (y*)? = y*.
        assert_eq!(optimized_pattern(r"(x+){1,}(y*){0,}"), "(x+)(y*)?");
    }

    #[test]
    fn capture_group_preserved_when_nested_repeat_simplified() {
        assert_eq!(optimized_pattern(r"(x+){1,}"), "(x+)");
    }

    #[test]
    fn non_greedy_nested_repeats_left_alone() {
        assert_eq!(optimized_pattern(r"(x+?){1,}"), "(x+?)+");
        assert_eq!(optimized_pattern(r"(x+){0,1}?"), "(x+)??");
    }

    #[test]
    fn non_capturing_group_nested_repeats_still_simplified() {
        // Non-capturing groups are transparent (unwrapped by the parser), so
        // they hit the bare-Repeat branch which has no outer_lo guard.
        // These are safe because there is no capture group to preserve.
        assert_eq!(optimized_pattern(r"(?:x+){1,}"), "x+");
        assert_eq!(optimized_pattern(r"(?:x*){0,}"), "x*");
        assert_eq!(optimized_pattern(r"(?:x+)+"), "x+");
        assert_eq!(optimized_pattern(r"(?:x*)*"), "x*");
        assert_eq!(optimized_pattern(r"(?:x*)+"), "x*");
        assert_eq!(optimized_pattern(r"(?:x+)*"), "x*");
    }

    #[test]
    fn bounded_quantifier_left_alone() {
        let max = usize::MAX;
        assert_eq!(
            super::can_simplify(1, max, true, 1, max, true),
            Some(super::QuantifierKind::OneOrMore)
        );
        assert_eq!(super::can_simplify(2, max, true, 1, max, true), None);
    }

    #[test]
    fn nested_repeat_does_not_affect_capture_group_fixup_return() {
        let mut tree = Expr::parse_tree(r"(x+){1,}").unwrap();
        let requires_capture_group_fixup = optimize(&mut tree);
        assert_eq!(requires_capture_group_fixup, false);
        assert_eq!(optimized_pattern(r"(x+){1,}"), "(x+)");
    }

    #[test]
    fn nested_repeats_from_oniguruma_adjacent_quantifiers_not_simplified() {
        // (x+){1,}{0,} — the outer {0,} has lo=0 so the capture group can be
        // skipped (→ None). Simplifying to (x*) would always enter the group.
        // With no backreferences, the trailing * is downgraded to ? since
        // (x+)* and (x+)? are equivalent (both match zero or more x) and
        // (x+)? is more efficient for the VM (single backtrack point).
        assert_eq!(
            optimized_pattern_with_flags(r"(x+){1,}{0,}", oniguruma_flags()),
            "(x+)?"
        );
    }

    #[test]
    fn backref_prevents_star_to_optional_downgrade() {
        // When there's a backreference, the * → ? downgrade must NOT happen
        // because the group participation count is observable via \1.
        let mut tree = Expr::parse_tree_with_flags(r"(x+){1,}{0,}\1", oniguruma_flags()).unwrap();
        optimize(&mut tree);
        // The inner {1,} is simplified to (x+), but the outer {0,} stays
        // as ZeroOrMore (not downgraded to Optional) because has_backrefs is true.
        assert_eq!(
            tree.expr,
            Expr::Concat(vec![
                Expr::Repeat {
                    child: Box::new(Expr::Group(Arc::new(Expr::Repeat {
                        child: Box::new(make_literal("x")),
                        lo: 1,
                        hi: usize::MAX,
                        greedy: true,
                    }))),
                    lo: 0,
                    hi: usize::MAX,
                    greedy: true,
                },
                Expr::Backref {
                    group: 1,
                    casei: false,
                },
            ])
        );
    }

    #[test]
    fn non_self_absorbing_group_not_downgraded() {
        // (x?){0,} — inner is Optional (lo=0, hi=1), not self-absorbing.
        // Even without backreferences, * is NOT downgraded to ? because
        // (x?)* = x* ≠ (x?)? = x? (they match different strings).
        assert_eq!(optimized_pattern(r"(x?){0,}"), "(x?)*");
    }

    #[test]
    fn ambiguous_concat_repeats_simplified_basic() {
        assert_eq!(optimized_pattern(r"\s*\w?\s*"), r"\s*(?:\w{1}\s*)?");
    }

    #[test]
    fn ambiguous_concat_repeats_simplified_with_bounded_middle() {
        // left=\w* (lo=0) < right=\w+ (lo=1): right is mandatory suffix, left+middle become optional prefix
        assert_eq!(
            optimized_pattern(r"foo\w*\s{0,5}\w+"),
            r"foo(?:\w*\s{1,5})?\w+"
        );
    }

    #[test]
    fn ambiguous_concat_repeats_simplified_with_nongreedy_middle() {
        // left=\w* (lo=0) < right=\w+ (lo=1): same as above with non-greedy middle
        assert_eq!(
            optimized_pattern(r"foo\w*\s{0,5}?\w+"),
            r"foo(?:\w*\s{1,5}?)?\w+"
        );
    }

    #[test]
    fn ambiguous_concat_repeats_simplified_with_plus_and_star() {
        // left=\s+ (lo=1) >= right=\s* (lo=0): left is mandatory prefix, right+middle become optional suffix
        assert_eq!(optimized_pattern(r"\s+\w{0,1}\s*"), r"\s+(?:\w{1}\s*)?");
        assert_eq!(optimized_pattern(r"^\s+\w{0,1}\s*$"), r"^\s+(?:\w{1}\s*)?$");
    }

    #[test]
    fn ambiguous_concat_repeats_inside_plus_simplified() {
        assert_eq!(optimized_pattern(r"(?:\s*\w?\s*)+"), r"\s*(?:\w{1}\s*)*");
    }

    #[test]
    fn ambiguous_concat_repeats_inside_star_simplified() {
        assert_eq!(
            optimized_pattern(r"(?:\s*\w?\s*)*"),
            r"(?:\s*(?:\w{1}\s*)*)?"
        );
    }

    #[test]
    fn ambiguous_concat_repeats_star_optional_plus_uses_suffix() {
        // left=\w* (lo=0) < right=\w+ (lo=1): the more restrictive side becomes the mandatory
        // suffix to avoid making the rewritten expression more permissive than the original.
        // \w*\.?\w+ cannot match an empty string (requires at least one word char from \w+),
        // so the rewritten form must also require at least one word char.
        assert_eq!(optimized_pattern(r"\w*\.?\w+"), r"(?:\w*\.{1})?\w+");
    }

    #[test]
    fn ambiguous_concat_repeats_plus_optional_star_uses_prefix() {
        // left=\w+ (lo=1) >= right=\w* (lo=0): left is the mandatory prefix
        assert_eq!(optimized_pattern(r"\w+\.?\w*"), r"\w+(?:\.{1}\w*)?");
    }
}
