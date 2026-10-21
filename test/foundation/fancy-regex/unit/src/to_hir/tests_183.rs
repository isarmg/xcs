#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze::{analyze, AnalyzeContext};
    use crate::{BytesMode, RegexOptions};
    use alloc::string::ToString;

    /// Patterns covering the easy subset: literals (plain, quoted, casei,
    /// non-ASCII), dots in all newline/CRLF modes, anchors in all line modes,
    /// concat/alt/group nesting, all quantifier shapes, classes and other
    /// delegated fragments, and mixed inline flags.
    const PATTERNS: &[&str] = &[
        "a",
        "abc",
        "a.c",
        ".",
        "(?s).",
        "(?s:.)x",
        "^abc$",
        "(?m)^abc$",
        "(?m:^)foo",
        "(?Rm)^x$",
        "(?i)abc",
        "(?i)aBc(?-i)d",
        "(?i)δΔ",
        "αβγ",
        "a+b*c?",
        "a{2,5}",
        "a{3}",
        "a{2,}?",
        "a+?b??",
        "(a)(b(c))",
        "(?:ab|cd)e",
        "a|b|",
        "(a|)",
        "()",
        "[a-z]+",
        r"\d\w\s",
        "[^a-c]{2,3}",
        r"\p{L}+",
        "(?i)[a-k]x",
        r"\x61\n\t",
        "(a+)(?:b|c)*",
        r"\.\*\+#",
        r"(?i)ſ",
        // Hard patterns must be skipped by the harness (info.hard).
        r"\bword\b",
        r"(a)\1",
    ];

    /// The translator must produce exactly the Hir the parser produces for
    /// the `to_str` serialization — or `None` where the parser errors, so the
    /// string path can report the error.
    fn check_oracle(bytes_mode: BytesMode) {
        for pattern in PATTERNS {
            let options = RegexOptions {
                bytes_mode,
                ..RegexOptions::default()
            };
            let Ok(tree) = crate::Expr::parse_tree_with_flags(pattern, options.compute_flags())
            else {
                continue;
            };
            let Ok(info) = analyze(&tree, AnalyzeContext::default()) else {
                continue;
            };
            if info.hard {
                continue;
            }
            let mut cooked = String::new();
            tree.expr.to_str(&mut cooked, 0);
            let unicode =
                options.syntaxc.get_unicode() && !matches!(options.bytes_mode, BytesMode::Ascii);
            let utf8 = matches!(options.bytes_mode, BytesMode::Unicode);
            let expected = regex_syntax::ParserBuilder::new()
                .utf8(utf8)
                .unicode(unicode)
                .build()
                .parse(&cooked);
            let got = expr_to_hir(&tree.expr, &mut HirCtx::new(unicode, utf8));
            match expected {
                Ok(hir) => assert_eq!(
                    Some(hir),
                    got,
                    "Hir mismatch for {:?} (cooked {:?}) in {:?} mode",
                    pattern,
                    cooked,
                    bytes_mode
                ),
                Err(_) => assert_eq!(
                    None, got,
                    "expected fallback for {:?} (cooked {:?}) in {:?} mode: the parser \
                     rejects it, so the translator must not accept it",
                    pattern, cooked, bytes_mode
                ),
            }
        }
    }

    #[test]
    fn oracle_unicode_mode() {
        check_oracle(BytesMode::Unicode);
    }

    #[test]
    fn oracle_unicode_bytes_mode() {
        check_oracle(BytesMode::UnicodeBytes);
    }

    #[test]
    fn oracle_ascii_mode() {
        check_oracle(BytesMode::Ascii);
    }

    #[test]
    fn group_numbering_matches_textual_order() {
        let tree = crate::Expr::parse_tree("(a)((b)(c))").unwrap();
        let hir = expr_to_hir(&tree.expr, &mut HirCtx::new(true, true)).unwrap();
        let cooked = {
            let mut s = String::new();
            tree.expr.to_str(&mut s, 0);
            s
        };
        let expected = regex_syntax::ParserBuilder::new().build().parse(&cooked);
        assert_eq!(expected.unwrap().to_string(), hir.to_string());
    }
}
