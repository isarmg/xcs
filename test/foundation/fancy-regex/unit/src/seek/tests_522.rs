#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze::{analyze, AnalyzeContext};
    use crate::compile::populate_group_info_map;
    use crate::optimize;
    use crate::Regex;

    /// Build the seek pattern for a regex string and return it.
    fn get_seek_pattern(re: &str) -> String {
        let mut tree = Expr::parse_tree(re).unwrap();
        let requires_capture_group_fixup = optimize(&mut tree);

        let info = analyze(
            &tree,
            AnalyzeContext {
                explicit_capture_group_0: requires_capture_group_fixup,
                ..AnalyzeContext::default()
            },
        )
        .unwrap();

        let mut group_info_map = Map::new();
        populate_group_info_map(&mut group_info_map, &info);
        let mut buf = String::new();
        build_seek_pattern(&info, &group_info_map, 0, &mut buf, 0);
        buf
    }

    #[test]
    fn seek_pattern_backref_no_anchor() {
        // Simple backref with no positional anchors: group body is inlined verbatim.
        // The `(?:...)` wrapping comes from Concat precedence handling when stripping groups.
        assert_eq!(get_seek_pattern(r"(abc)\1"), "(?:abc)(?:abc)");
        assert_eq!(
            get_seek_pattern(r"(?i)(abc)\1"),
            "(?:(?i:a)(?i:b)(?i:c))(?i:(?i:a)(?i:b)(?i:c))"
        );
    }

    #[test]
    fn seek_pattern_backref_with_start_anchor() {
        // Backref to a group whose body starts with `^`: the anchor should be dropped from the
        // inlined backref position but kept at the group definition site.
        // `(^a)\1` — seek = `(?:^a)` (group) + `(?:a)` (backref, anchor dropped)
        assert_eq!(get_seek_pattern(r"(^a)\1"), "(?:^a)(?:a)");
    }

    #[test]
    fn seek_pattern_backref_with_start_anchor_variable_length() {
        // The group has a start anchor plus a variable-length content.
        // Anchor is dropped when inlining for the backref.
        assert_eq!(get_seek_pattern(r"(^a+)\1"), "(?:^a+)(?:a+)");
    }

    #[test]
    fn seek_pattern_backref_with_end_anchor() {
        // Backref to a group ending with `$`: the anchor is dropped in the inline position.
        assert_eq!(get_seek_pattern(r"(a$)\1"), "(?:a$)(?:a)");
    }

    #[test]
    fn seek_pattern_backref_only_anchor() {
        // Backref to a group that is only a positional anchor.
        // The group emits `^` (min_size=0); the backref drops `^` and emits nothing.
        assert_eq!(get_seek_pattern(r"(^)\1"), "^");
    }

    #[test]
    fn seek_pattern_lookahead_dropped() {
        // Lookahead is zero-width and is dropped; only the literal is kept.
        assert_eq!(get_seek_pattern(r"(?=foo)bar"), "bar");
    }

    #[test]
    fn seek_pattern_casei_literal_backref_with_anchor() {
        // Group contains a case-insensitive literal and a start anchor.
        // The anchor is dropped during backref inlining; the casei literal emits (?i:...).
        assert_eq!(get_seek_pattern(r"(?i:(^a))\1"), "(?:^(?i:a))(?:(?i:a))");
    }

    #[test]
    fn seek_pattern_easy_expr_preserved() {
        // Easy (non-hard) expressions without capture groups are serialised verbatim via to_str.
        assert_eq!(get_seek_pattern(r"abc"), "abc");
        assert_eq!(get_seek_pattern(r"a|b"), "a|b");
        assert_eq!(get_seek_pattern(r"a+b*c?"), "a+b*c?");
    }

    #[test]
    fn seek_pattern_easy_expr_with_capture_group_strips_group_wrapper() {
        // Easy expressions with capture groups drop the capture group wrappings.
        assert_eq!(get_seek_pattern(r"(abc)"), "abc");
        assert_eq!(
            get_seek_pattern(r"(abc(def))(?<named>ghi)"),
            "(?:abc(?:def))(?:ghi)"
        );
    }

    #[test]
    fn seek_pattern_optimized_easy_expr_strips_group_wrapper() {
        // Optimized patterns lose the capture group wrappings
        assert_eq!(get_seek_pattern(r"(?=abc)"), "(?:abc)");
        assert_eq!(
            get_seek_pattern(r"(h)(e)(l)(l)(o)\s*(?=world)"),
            r"(?:hello\s*)(?:world)"
        );
    }

    #[test]
    fn seek_pattern_end_text_ignore_trailing_newlines_non_crlf() {
        // \Z (non-CRLF mode) — approximate with `\n*$` so the seek only visits positions
        // near end-of-text.
        assert_eq!(get_seek_pattern(r"abc\Z"), r"abc\n*$");
    }

    #[test]
    fn seek_pattern_end_text_ignore_trailing_newlines_crlf() {
        // \Z in CRLF mode — approximate with `(?:\r?\n)*$`.
        assert_eq!(get_seek_pattern(r"(?R)abc\Z"), r"abc[\r\n]*$");
    }

    #[test]
    fn seek_pattern_end_text_ignore_trailing_newlines_only() {
        // \Z alone (hard assertion) — just the seek pattern with no surrounding literal.
        assert_eq!(get_seek_pattern(r"\Z"), r"\n*$");
    }

    #[test]
    fn seek_pattern_backref_with_end_text_ignore_trailing_newlines() {
        // Backref to a group that ends with \Z: \Z is a positional anchor and should be
        // dropped when inlining the group body for the backref.
        // `(a\Z)\1` — seek = `(?:a\n*$)` (group) + `(?:a\n*)` (backref, \Z anchor dropped, newline matching kept)
        assert_eq!(get_seek_pattern(r"(a\Z)\1"), r"(?:a\n*$)(?:a\n*)");
    }

    #[test]
    fn seek_pattern_subroutine_call_inlined() {
        assert_eq!(
            get_seek_pattern(r"([A-Z][a-z0-9]*)::\g<1>"),
            r"(?:[A-Z][a-z0-9]*)::(?:[A-Z][a-z0-9]*)"
        );
    }

    #[test]
    fn seek_pattern_subroutine_call_inlined_and_anchors_are_preserved() {
        assert_eq!(
            get_seek_pattern(r"((?m:^)[A-Z][a-z0-9]*)\n\g<1>"),
            "(?:(?m:^)[A-Z][a-z0-9]*)\\n(?:(?m:^)[A-Z][a-z0-9]*)"
        );
    }

    #[test]
    fn seek_pattern_self_referential_backref_is_bounded() {
        // Inlining a backref whose target transitively references the backref's
        // own ancestors would expand exponentially with recursion depth if we
        // didn't have cycle detection or a length cap to cause it to fall back
        // to a permissive placeholder instead of continuing to recurse.
        // Together these help to keep the seek pattern small (and, therefore, compilation
        // fast) rather than producing a multi-megabyte string which would likely not help
        // to narrow candidate start positions much better anyway.
        let seek = get_seek_pattern(r"(end)(\s+(function))?(\s+((\3|\4|\5)))?");
        assert!(
            seek.len() <= MAX_SEEK_PATTERN_LEN,
            "seek pattern should stay bounded, got {} bytes",
            seek.len()
        );
        // Compiling it must succeed and stay well-formed.
        Regex::new(r"(end)(\s+(function))?(\s+((\3|\4|\5)))?").unwrap();
    }
}
