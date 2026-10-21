#[cfg(test)]
mod tests {
    use alloc::{
        boxed::Box,
        string::{String, ToString},
    };

    use crate::ParserBuilder;

    use super::*;

    fn roundtrip(given: &str, expected: &str) {
        roundtrip_with(|b| b, given, expected);
    }

    fn roundtrip_bytes(given: &str, expected: &str) {
        roundtrip_with(|b| b.utf8(false), given, expected);
    }

    fn roundtrip_with<F>(mut f: F, given: &str, expected: &str)
    where
        F: FnMut(&mut ParserBuilder) -> &mut ParserBuilder,
    {
        let mut builder = ParserBuilder::new();
        f(&mut builder);
        let hir = builder.build().parse(given).unwrap();

        let mut printer = Printer::new();
        let mut dst = String::new();
        printer.print(&hir, &mut dst).unwrap();

        // Check that the result is actually valid.
        builder.build().parse(&dst).unwrap();

        assert_eq!(expected, dst);
    }

    #[test]
    fn print_literal() {
        roundtrip("a", "a");
        roundtrip(r"\xff", "\u{FF}");
        roundtrip_bytes(r"\xff", "\u{FF}");
        roundtrip_bytes(r"(?-u)\xff", r"(?-u:\xFF)");
        roundtrip("☃", "☃");
    }

    #[test]
    fn print_class() {
        roundtrip(r"[a]", r"a");
        roundtrip(r"[ab]", r"[ab]");
        roundtrip(r"[a-z]", r"[a-z]");
        roundtrip(r"[a-z--b-c--x-y]", r"[ad-wz]");
        roundtrip(r"[^\x01-\u{10FFFF}]", "\u{0}");
        roundtrip(r"[-]", r"\-");
        roundtrip(r"[☃-⛄]", r"[☃-⛄]");

        roundtrip(r"(?-u)[a]", r"a");
        roundtrip(r"(?-u)[ab]", r"(?-u:[ab])");
        roundtrip(r"(?-u)[a-z]", r"(?-u:[a-z])");
        roundtrip_bytes(r"(?-u)[a-\xFF]", r"(?-u:[a-\xFF])");

        // The following test that the printer escapes meta characters
        // in character classes.
        roundtrip(r"[\[]", r"\[");
        roundtrip(r"[Z-_]", r"[Z-_]");
        roundtrip(r"[Z-_--Z]", r"[\[-_]");

        // The following test that the printer escapes meta characters
        // in byte oriented character classes.
        roundtrip_bytes(r"(?-u)[\[]", r"\[");
        roundtrip_bytes(r"(?-u)[Z-_]", r"(?-u:[Z-_])");
        roundtrip_bytes(r"(?-u)[Z-_--Z]", r"(?-u:[\[-_])");

        // This tests that an empty character class is correctly roundtripped.
        #[cfg(feature = "unicode-gencat")]
        roundtrip(r"\P{any}", r"[a&&b]");
        roundtrip_bytes(r"(?-u)[^\x00-\xFF]", r"[a&&b]");
    }

    #[test]
    fn print_anchor() {
        roundtrip(r"^", r"\A");
        roundtrip(r"$", r"\z");
        roundtrip(r"(?m)^", r"(?m:^)");
        roundtrip(r"(?m)$", r"(?m:$)");
    }

    #[test]
    fn print_word_boundary() {
        roundtrip(r"\b", r"\b");
        roundtrip(r"\B", r"\B");
        roundtrip(r"(?-u)\b", r"(?-u:\b)");
        roundtrip_bytes(r"(?-u)\B", r"(?-u:\B)");
    }

    #[test]
    fn print_repetition() {
        roundtrip("a?", "a?");
        roundtrip("a??", "a??");
        roundtrip("(?U)a?", "a??");

        roundtrip("a*", "a*");
        roundtrip("a*?", "a*?");
        roundtrip("(?U)a*", "a*?");

        roundtrip("a+", "a+");
        roundtrip("a+?", "a+?");
        roundtrip("(?U)a+", "a+?");

        roundtrip("a{1}", "a");
        roundtrip("a{2}", "a{2}");
        roundtrip("a{1,}", "a+");
        roundtrip("a{1,5}", "a{1,5}");
        roundtrip("a{1}?", "a");
        roundtrip("a{2}?", "a{2}");
        roundtrip("a{1,}?", "a+?");
        roundtrip("a{1,5}?", "a{1,5}?");
        roundtrip("(?U)a{1}", "a");
        roundtrip("(?U)a{2}", "a{2}");
        roundtrip("(?U)a{1,}", "a+?");
        roundtrip("(?U)a{1,5}", "a{1,5}?");

        // Test that various zero-length repetitions always translate to an
        // empty regex. This is more a property of HIR's smart constructors
        // than the printer though.
        roundtrip("a{0}", "(?:)");
        roundtrip("(?:ab){0}", "(?:)");
        #[cfg(feature = "unicode-gencat")]
        {
            roundtrip(r"\p{any}{0}", "(?:)");
            roundtrip(r"\P{any}{0}", "(?:)");
        }
    }

    #[test]
    fn print_group() {
        roundtrip("()", "((?:))");
        roundtrip("(?P<foo>)", "(?P<foo>(?:))");
        roundtrip("(?:)", "(?:)");

        roundtrip("(a)", "(a)");
        roundtrip("(?P<foo>a)", "(?P<foo>a)");
        roundtrip("(?:a)", "a");

        roundtrip("((((a))))", "((((a))))");
    }

    #[test]
    fn print_alternation() {
        roundtrip("|", "(?:(?:)|(?:))");
        roundtrip("||", "(?:(?:)|(?:)|(?:))");

        roundtrip("a|b", "[ab]");
        roundtrip("ab|cd", "(?:(?:ab)|(?:cd))");
        roundtrip("a|b|c", "[a-c]");
        roundtrip("ab|cd|ef", "(?:(?:ab)|(?:cd)|(?:ef))");
        roundtrip("foo|bar|quux", "(?:(?:foo)|(?:bar)|(?:quux))");
    }

    // This is a regression test that stresses a peculiarity of how the HIR
    // is both constructed and printed. Namely, it is legal for a repetition
    // to directly contain a concatenation. This particular construct isn't
    // really possible to build from the concrete syntax directly, since you'd
    // be forced to put the concatenation into (at least) a non-capturing
    // group. Concurrently, the printer doesn't consider this case and just
    // kind of naively prints the child expression and tacks on the repetition
    // operator.
    //
    // As a result, if you attached '+' to a 'concat(a, b)', the printer gives
    // you 'ab+', but clearly it really should be '(?:ab)+'.
    //
    // This bug isn't easy to surface because most ways of building an HIR
    // come directly from the concrete syntax, and as mentioned above, it just
    // isn't possible to build this kind of HIR from the concrete syntax.
    // Nevertheless, this is definitely a bug.
    //
    // See: https://github.com/rust-lang/regex/issues/731
    #[test]
    fn regression_repetition_concat() {
        let expr = Hir::concat(alloc::vec![
            Hir::literal("x".as_bytes()),
            Hir::repetition(hir::Repetition {
                min: 1,
                max: None,
                greedy: true,
                sub: Box::new(Hir::literal("ab".as_bytes())),
            }),
            Hir::literal("y".as_bytes()),
        ]);
        assert_eq!(r"(?:x(?:ab)+y)", expr.to_string());

        let expr = Hir::concat(alloc::vec![
            Hir::look(hir::Look::Start),
            Hir::repetition(hir::Repetition {
                min: 1,
                max: None,
                greedy: true,
                sub: Box::new(Hir::concat(alloc::vec![
                    Hir::look(hir::Look::Start),
                    Hir::look(hir::Look::End),
                ])),
            }),
            Hir::look(hir::Look::End),
        ]);
        assert_eq!(r"(?:\A\A\z\z)", expr.to_string());
    }

    // Just like regression_repetition_concat, but with the repetition using
    // an alternation as a child expression instead.
    //
    // See: https://github.com/rust-lang/regex/issues/731
    #[test]
    fn regression_repetition_alternation() {
        let expr = Hir::concat(alloc::vec![
            Hir::literal("ab".as_bytes()),
            Hir::repetition(hir::Repetition {
                min: 1,
                max: None,
                greedy: true,
                sub: Box::new(Hir::alternation(alloc::vec![
                    Hir::literal("cd".as_bytes()),
                    Hir::literal("ef".as_bytes()),
                ])),
            }),
            Hir::literal("gh".as_bytes()),
        ]);
        assert_eq!(r"(?:(?:ab)(?:(?:cd)|(?:ef))+(?:gh))", expr.to_string());

        let expr = Hir::concat(alloc::vec![
            Hir::look(hir::Look::Start),
            Hir::repetition(hir::Repetition {
                min: 1,
                max: None,
                greedy: true,
                sub: Box::new(Hir::alternation(alloc::vec![
                    Hir::look(hir::Look::Start),
                    Hir::look(hir::Look::End),
                ])),
            }),
            Hir::look(hir::Look::End),
        ]);
        assert_eq!(r"(?:\A(?:\A|\z)\z)", expr.to_string());
    }

    // This regression test is very similar in flavor to
    // regression_repetition_concat in that the root of the issue lies in a
    // peculiarity of how the HIR is represented and how the printer writes it
    // out. Like the other regression, this one is also rooted in the fact that
    // you can't produce the peculiar HIR from the concrete syntax. Namely, you
    // just can't have a 'concat(a, alt(b, c))' because the 'alt' will normally
    // be in (at least) a non-capturing group. Why? Because the '|' has very
    // low precedence (lower that concatenation), and so something like 'ab|c'
    // is actually 'alt(ab, c)'.
    //
    // See: https://github.com/rust-lang/regex/issues/516
    #[test]
    fn regression_alternation_concat() {
        let expr = Hir::concat(alloc::vec![
            Hir::literal("ab".as_bytes()),
            Hir::alternation(alloc::vec![
                Hir::literal("mn".as_bytes()),
                Hir::literal("xy".as_bytes()),
            ]),
        ]);
        assert_eq!(r"(?:(?:ab)(?:(?:mn)|(?:xy)))", expr.to_string());

        let expr = Hir::concat(alloc::vec![
            Hir::look(hir::Look::Start),
            Hir::alternation(alloc::vec![
                Hir::look(hir::Look::Start),
                Hir::look(hir::Look::End),
            ]),
        ]);
        assert_eq!(r"(?:\A(?:\A|\z))", expr.to_string());
    }
}
