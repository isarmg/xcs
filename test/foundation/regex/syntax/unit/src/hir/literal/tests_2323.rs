#[cfg(test)]
mod tests {
    use super::*;

    fn parse(pattern: &str) -> Hir {
        crate::syntax::ParserBuilder::new().utf8(false).build().parse(pattern).unwrap()
    }

    fn prefixes(pattern: &str) -> Seq {
        Extractor::new().kind(ExtractKind::Prefix).extract(&parse(pattern))
    }

    fn suffixes(pattern: &str) -> Seq {
        Extractor::new().kind(ExtractKind::Suffix).extract(&parse(pattern))
    }

    fn e(pattern: &str) -> (Seq, Seq) {
        (prefixes(pattern), suffixes(pattern))
    }

    #[allow(non_snake_case)]
    fn E(x: &str) -> Literal {
        Literal::exact(x.as_bytes())
    }

    #[allow(non_snake_case)]
    fn I(x: &str) -> Literal {
        Literal::inexact(x.as_bytes())
    }

    fn seq<I: IntoIterator<Item = Literal>>(it: I) -> Seq {
        Seq::from_iter(it)
    }

    fn infinite() -> (Seq, Seq) {
        (Seq::infinite(), Seq::infinite())
    }

    fn inexact<I1, I2>(it1: I1, it2: I2) -> (Seq, Seq)
    where
        I1: IntoIterator<Item = Literal>,
        I2: IntoIterator<Item = Literal>,
    {
        (Seq::from_iter(it1), Seq::from_iter(it2))
    }

    fn exact<B: AsRef<[u8]>, I: IntoIterator<Item = B>>(it: I) -> (Seq, Seq) {
        let s1 = Seq::new(it);
        let s2 = s1.clone();
        (s1, s2)
    }

    fn opt<B: AsRef<[u8]>, I: IntoIterator<Item = B>>(it: I) -> (Seq, Seq) {
        let (mut p, mut s) = exact(it);
        p.optimize_for_prefix_by_preference();
        s.optimize_for_suffix_by_preference();
        (p, s)
    }

    #[test]
    fn literal() {
        assert_eq!(exact(["a"]), e("a"));
        assert_eq!(exact(["aaaaa"]), e("aaaaa"));
        assert_eq!(exact(["A", "a"]), e("(?i-u)a"));
        assert_eq!(exact(["AB", "Ab", "aB", "ab"]), e("(?i-u)ab"));
        assert_eq!(exact(["abC", "abc"]), e("ab(?i-u)c"));

        assert_eq!(exact([b"\xFF"]), e(r"(?-u:\xFF)"));

        #[cfg(feature = "unicode-case")]
        {
            assert_eq!(exact(["☃"]), e("☃"));
            assert_eq!(exact(["☃"]), e("(?i)☃"));
            assert_eq!(exact(["☃☃☃☃☃"]), e("☃☃☃☃☃"));

            assert_eq!(exact(["Δ"]), e("Δ"));
            assert_eq!(exact(["δ"]), e("δ"));
            assert_eq!(exact(["Δ", "δ"]), e("(?i)Δ"));
            assert_eq!(exact(["Δ", "δ"]), e("(?i)δ"));

            assert_eq!(exact(["S", "s", "ſ"]), e("(?i)S"));
            assert_eq!(exact(["S", "s", "ſ"]), e("(?i)s"));
            assert_eq!(exact(["S", "s", "ſ"]), e("(?i)ſ"));
        }

        let letters = "ͱͳͷΐάέήίΰαβγδεζηθικλμνξοπρςστυφχψωϊϋ";
        assert_eq!(exact([letters]), e(letters));
    }

    #[test]
    fn class() {
        assert_eq!(exact(["a", "b", "c"]), e("[abc]"));
        assert_eq!(exact(["a1b", "a2b", "a3b"]), e("a[123]b"));
        assert_eq!(exact(["δ", "ε"]), e("[εδ]"));
        #[cfg(feature = "unicode-case")]
        {
            assert_eq!(exact(["Δ", "Ε", "δ", "ε", "ϵ"]), e(r"(?i)[εδ]"));
        }
    }

    #[test]
    fn look() {
        assert_eq!(exact(["ab"]), e(r"a\Ab"));
        assert_eq!(exact(["ab"]), e(r"a\zb"));
        assert_eq!(exact(["ab"]), e(r"a(?m:^)b"));
        assert_eq!(exact(["ab"]), e(r"a(?m:$)b"));
        assert_eq!(exact(["ab"]), e(r"a\bb"));
        assert_eq!(exact(["ab"]), e(r"a\Bb"));
        assert_eq!(exact(["ab"]), e(r"a(?-u:\b)b"));
        assert_eq!(exact(["ab"]), e(r"a(?-u:\B)b"));

        assert_eq!(exact(["ab"]), e(r"^ab"));
        assert_eq!(exact(["ab"]), e(r"$ab"));
        assert_eq!(exact(["ab"]), e(r"(?m:^)ab"));
        assert_eq!(exact(["ab"]), e(r"(?m:$)ab"));
        assert_eq!(exact(["ab"]), e(r"\bab"));
        assert_eq!(exact(["ab"]), e(r"\Bab"));
        assert_eq!(exact(["ab"]), e(r"(?-u:\b)ab"));
        assert_eq!(exact(["ab"]), e(r"(?-u:\B)ab"));

        assert_eq!(exact(["ab"]), e(r"ab^"));
        assert_eq!(exact(["ab"]), e(r"ab$"));
        assert_eq!(exact(["ab"]), e(r"ab(?m:^)"));
        assert_eq!(exact(["ab"]), e(r"ab(?m:$)"));
        assert_eq!(exact(["ab"]), e(r"ab\b"));
        assert_eq!(exact(["ab"]), e(r"ab\B"));
        assert_eq!(exact(["ab"]), e(r"ab(?-u:\b)"));
        assert_eq!(exact(["ab"]), e(r"ab(?-u:\B)"));

        let expected = (seq([I("aZ"), E("ab")]), seq([I("Zb"), E("ab")]));
        assert_eq!(expected, e(r"^aZ*b"));
    }

    #[test]
    fn repetition() {
        assert_eq!(exact(["a", ""]), e(r"a?"));
        assert_eq!(exact(["", "a"]), e(r"a??"));
        assert_eq!(inexact([I("a"), E("")], [I("a"), E("")]), e(r"a*"));
        assert_eq!(inexact([E(""), I("a")], [E(""), I("a")]), e(r"a*?"));
        assert_eq!(inexact([I("a")], [I("a")]), e(r"a+"));
        assert_eq!(inexact([I("a")], [I("a")]), e(r"(a+)+"));

        assert_eq!(exact(["ab"]), e(r"aZ{0}b"));
        assert_eq!(exact(["aZb", "ab"]), e(r"aZ?b"));
        assert_eq!(exact(["ab", "aZb"]), e(r"aZ??b"));
        assert_eq!(
            inexact([I("aZ"), E("ab")], [I("Zb"), E("ab")]),
            e(r"aZ*b")
        );
        assert_eq!(
            inexact([E("ab"), I("aZ")], [E("ab"), I("Zb")]),
            e(r"aZ*?b")
        );
        assert_eq!(inexact([I("aZ")], [I("Zb")]), e(r"aZ+b"));
        assert_eq!(inexact([I("aZ")], [I("Zb")]), e(r"aZ+?b"));

        assert_eq!(exact(["aZZb"]), e(r"aZ{2}b"));
        assert_eq!(inexact([I("aZZ")], [I("ZZb")]), e(r"aZ{2,3}b"));

        assert_eq!(exact(["abc", ""]), e(r"(abc)?"));
        assert_eq!(exact(["", "abc"]), e(r"(abc)??"));

        assert_eq!(inexact([I("a"), E("b")], [I("ab"), E("b")]), e(r"a*b"));
        assert_eq!(inexact([E("b"), I("a")], [E("b"), I("ab")]), e(r"a*?b"));
        assert_eq!(inexact([I("ab")], [I("b")]), e(r"ab+"));
        assert_eq!(inexact([I("a"), I("b")], [I("b")]), e(r"a*b+"));

        // FIXME: The suffixes for this don't look quite right to me. I think
        // the right suffixes would be: [I(ac), I(bc), E(c)]. The main issue I
        // think is that suffixes are computed by iterating over concatenations
        // in reverse, and then [bc, ac, c] ordering is indeed correct from
        // that perspective. We also test a few more equivalent regexes, and
        // we get the same result, so it is consistent at least I suppose.
        //
        // The reason why this isn't an issue is that it only messes up
        // preference order, and currently, suffixes are never used in a
        // context where preference order matters. For prefixes it matters
        // because we sometimes want to use prefilters without confirmation
        // when all of the literals are exact (and there's no look-around). But
        // we never do that for suffixes. Any time we use suffixes, we always
        // include a confirmation step. If that ever changes, then it's likely
        // this bug will need to be fixed, but last time I looked, it appears
        // hard to do so.
        assert_eq!(
            inexact([I("a"), I("b"), E("c")], [I("bc"), I("ac"), E("c")]),
            e(r"a*b*c")
        );
        assert_eq!(
            inexact([I("a"), I("b"), E("c")], [I("bc"), I("ac"), E("c")]),
            e(r"(a+)?(b+)?c")
        );
        assert_eq!(
            inexact([I("a"), I("b"), E("c")], [I("bc"), I("ac"), E("c")]),
            e(r"(a+|)(b+|)c")
        );
        // A few more similarish but not identical regexes. These may have a
        // similar problem as above.
        assert_eq!(
            inexact(
                [I("a"), I("b"), I("c"), E("")],
                [I("c"), I("b"), I("a"), E("")]
            ),
            e(r"a*b*c*")
        );
        assert_eq!(inexact([I("a"), I("b"), I("c")], [I("c")]), e(r"a*b*c+"));
        assert_eq!(inexact([I("a"), I("b")], [I("bc")]), e(r"a*b+c"));
        assert_eq!(inexact([I("a"), I("b")], [I("c"), I("b")]), e(r"a*b+c*"));
        assert_eq!(inexact([I("ab"), E("a")], [I("b"), E("a")]), e(r"ab*"));
        assert_eq!(
            inexact([I("ab"), E("ac")], [I("bc"), E("ac")]),
            e(r"ab*c")
        );
        assert_eq!(inexact([I("ab")], [I("b")]), e(r"ab+"));
        assert_eq!(inexact([I("ab")], [I("bc")]), e(r"ab+c"));

        assert_eq!(
            inexact([I("z"), E("azb")], [I("zazb"), E("azb")]),
            e(r"z*azb")
        );

        let expected =
            exact(["aaa", "aab", "aba", "abb", "baa", "bab", "bba", "bbb"]);
        assert_eq!(expected, e(r"[ab]{3}"));
        let expected = inexact(
            [
                I("aaa"),
                I("aab"),
                I("aba"),
                I("abb"),
                I("baa"),
                I("bab"),
                I("bba"),
                I("bbb"),
            ],
            [
                I("aaa"),
                I("aab"),
                I("aba"),
                I("abb"),
                I("baa"),
                I("bab"),
                I("bba"),
                I("bbb"),
            ],
        );
        assert_eq!(expected, e(r"[ab]{3,4}"));
    }

    #[test]
    fn concat() {
        let empty: [&str; 0] = [];

        assert_eq!(exact(["abcxyz"]), e(r"abc()xyz"));
        assert_eq!(exact(["abcxyz"]), e(r"(abc)(xyz)"));
        assert_eq!(exact(["abcmnoxyz"]), e(r"abc()mno()xyz"));
        assert_eq!(exact(empty), e(r"abc[a&&b]xyz"));
        assert_eq!(exact(["abcxyz"]), e(r"abc[a&&b]*xyz"));
    }

    #[test]
    fn alternation() {
        assert_eq!(exact(["abc", "mno", "xyz"]), e(r"abc|mno|xyz"));
        assert_eq!(
            inexact(
                [E("abc"), I("mZ"), E("mo"), E("xyz")],
                [E("abc"), I("Zo"), E("mo"), E("xyz")]
            ),
            e(r"abc|mZ*o|xyz")
        );
        assert_eq!(exact(["abc", "xyz"]), e(r"abc|M[a&&b]N|xyz"));
        assert_eq!(exact(["abc", "MN", "xyz"]), e(r"abc|M[a&&b]*N|xyz"));

        assert_eq!(exact(["aaa", "aaaaa"]), e(r"(?:|aa)aaa"));
        assert_eq!(
            inexact(
                [I("aaa"), E(""), I("aaaaa"), E("aa")],
                [I("aaa"), E(""), E("aa")]
            ),
            e(r"(?:|aa)(?:aaa)*")
        );
        assert_eq!(
            inexact(
                [E(""), I("aaa"), E("aa"), I("aaaaa")],
                [E(""), I("aaa"), E("aa")]
            ),
            e(r"(?:|aa)(?:aaa)*?")
        );

        assert_eq!(
            inexact([E("a"), I("b"), E("")], [E("a"), I("b"), E("")]),
            e(r"a|b*")
        );
        assert_eq!(inexact([E("a"), I("b")], [E("a"), I("b")]), e(r"a|b+"));

        assert_eq!(
            inexact([I("a"), E("b"), E("c")], [I("ab"), E("b"), E("c")]),
            e(r"a*b|c")
        );

        assert_eq!(
            inexact(
                [E("a"), E("b"), I("c"), E("")],
                [E("a"), E("b"), I("c"), E("")]
            ),
            e(r"a|(?:b|c*)")
        );

        assert_eq!(
            inexact(
                [I("a"), I("b"), E("c"), I("a"), I("ab"), E("c")],
                [I("ac"), I("bc"), E("c"), I("ac"), I("abc"), E("c")],
            ),
            e(r"(a|b)*c|(a|ab)*c")
        );

        assert_eq!(
            exact(["abef", "abgh", "cdef", "cdgh"]),
            e(r"(ab|cd)(ef|gh)")
        );
        assert_eq!(
            exact([
                "abefij", "abefkl", "abghij", "abghkl", "cdefij", "cdefkl",
                "cdghij", "cdghkl",
            ]),
            e(r"(ab|cd)(ef|gh)(ij|kl)")
        );

        assert_eq!(inexact([E("abab")], [E("abab")]), e(r"(ab){2}"));

        assert_eq!(inexact([I("abab")], [I("abab")]), e(r"(ab){2,3}"));

        assert_eq!(inexact([I("abab")], [I("abab")]), e(r"(ab){2,}"));
    }

    #[test]
    fn impossible() {
        let empty: [&str; 0] = [];

        assert_eq!(exact(empty), e(r"[a&&b]"));
        assert_eq!(exact(empty), e(r"a[a&&b]"));
        assert_eq!(exact(empty), e(r"[a&&b]b"));
        assert_eq!(exact(empty), e(r"a[a&&b]b"));
        assert_eq!(exact(["a", "b"]), e(r"a|[a&&b]|b"));
        assert_eq!(exact(["a", "b"]), e(r"a|c[a&&b]|b"));
        assert_eq!(exact(["a", "b"]), e(r"a|[a&&b]d|b"));
        assert_eq!(exact(["a", "b"]), e(r"a|c[a&&b]d|b"));
        assert_eq!(exact([""]), e(r"[a&&b]*"));
        assert_eq!(exact(["MN"]), e(r"M[a&&b]*N"));
    }

    // This tests patterns that contain something that defeats literal
    // detection, usually because it would blow some limit on the total number
    // of literals that can be returned.
    //
    // The main idea is that when literal extraction sees something that
    // it knows will blow a limit, it replaces it with a marker that says
    // "any literal will match here." While not necessarily true, the
    // over-estimation is just fine for the purposes of literal extraction,
    // because the imprecision doesn't matter: too big is too big.
    //
    // This is one of the trickier parts of literal extraction, since we need
    // to make sure all of our literal extraction operations correctly compose
    // with the markers.
    #[test]
    fn anything() {
        assert_eq!(infinite(), e(r"."));
        assert_eq!(infinite(), e(r"(?s)."));
        assert_eq!(infinite(), e(r"[A-Za-z]"));
        assert_eq!(infinite(), e(r"[A-Z]"));
        assert_eq!(exact([""]), e(r"[A-Z]{0}"));
        assert_eq!(infinite(), e(r"[A-Z]?"));
        assert_eq!(infinite(), e(r"[A-Z]*"));
        assert_eq!(infinite(), e(r"[A-Z]+"));
        assert_eq!((seq([I("1")]), Seq::infinite()), e(r"1[A-Z]"));
        assert_eq!((seq([I("1")]), seq([I("2")])), e(r"1[A-Z]2"));
        assert_eq!((Seq::infinite(), seq([I("123")])), e(r"[A-Z]+123"));
        assert_eq!(infinite(), e(r"[A-Z]+123[A-Z]+"));
        assert_eq!(infinite(), e(r"1|[A-Z]|3"));
        assert_eq!(
            (seq([E("1"), I("2"), E("3")]), Seq::infinite()),
            e(r"1|2[A-Z]|3"),
        );
        assert_eq!(
            (Seq::infinite(), seq([E("1"), I("2"), E("3")])),
            e(r"1|[A-Z]2|3"),
        );
        assert_eq!(
            (seq([E("1"), I("2"), E("4")]), seq([E("1"), I("3"), E("4")])),
            e(r"1|2[A-Z]3|4"),
        );
        assert_eq!((Seq::infinite(), seq([I("2")])), e(r"(?:|1)[A-Z]2"));
        assert_eq!(inexact([I("a")], [I("z")]), e(r"a.z"));
    }

    // Like the 'anything' test, but it uses smaller limits in order to test
    // the logic for effectively aborting literal extraction when the seqs get
    // too big.
    #[test]
    fn anything_small_limits() {
        fn prefixes(pattern: &str) -> Seq {
            Extractor::new()
                .kind(ExtractKind::Prefix)
                .limit_total(10)
                .extract(&parse(pattern))
        }

        fn suffixes(pattern: &str) -> Seq {
            Extractor::new()
                .kind(ExtractKind::Suffix)
                .limit_total(10)
                .extract(&parse(pattern))
        }

        fn e(pattern: &str) -> (Seq, Seq) {
            (prefixes(pattern), suffixes(pattern))
        }

        assert_eq!(
            (
                seq([
                    I("aaa"),
                    I("aab"),
                    I("aba"),
                    I("abb"),
                    I("baa"),
                    I("bab"),
                    I("bba"),
                    I("bbb")
                ]),
                seq([
                    I("aaa"),
                    I("aab"),
                    I("aba"),
                    I("abb"),
                    I("baa"),
                    I("bab"),
                    I("bba"),
                    I("bbb")
                ])
            ),
            e(r"[ab]{3}{3}")
        );

        assert_eq!(infinite(), e(r"ab|cd|ef|gh|ij|kl|mn|op|qr|st|uv|wx|yz"));
    }

    #[test]
    fn empty() {
        assert_eq!(exact([""]), e(r""));
        assert_eq!(exact([""]), e(r"^"));
        assert_eq!(exact([""]), e(r"$"));
        assert_eq!(exact([""]), e(r"(?m:^)"));
        assert_eq!(exact([""]), e(r"(?m:$)"));
        assert_eq!(exact([""]), e(r"\b"));
        assert_eq!(exact([""]), e(r"\B"));
        assert_eq!(exact([""]), e(r"(?-u:\b)"));
        assert_eq!(exact([""]), e(r"(?-u:\B)"));
    }

    #[test]
    fn odds_and_ends() {
        assert_eq!((Seq::infinite(), seq([I("a")])), e(r".a"));
        assert_eq!((seq([I("a")]), Seq::infinite()), e(r"a."));
        assert_eq!(infinite(), e(r"a|."));
        assert_eq!(infinite(), e(r".|a"));

        let pat = r"M[ou]'?am+[ae]r .*([AEae]l[- ])?[GKQ]h?[aeu]+([dtz][dhz]?)+af[iy]";
        let expected = inexact(
            ["Mo'am", "Moam", "Mu'am", "Muam"].map(I),
            [
                "ddafi", "ddafy", "dhafi", "dhafy", "dzafi", "dzafy", "dafi",
                "dafy", "tdafi", "tdafy", "thafi", "thafy", "tzafi", "tzafy",
                "tafi", "tafy", "zdafi", "zdafy", "zhafi", "zhafy", "zzafi",
                "zzafy", "zafi", "zafy",
            ]
            .map(I),
        );
        assert_eq!(expected, e(pat));

        assert_eq!(
            (seq(["fn is_", "fn as_"].map(I)), Seq::infinite()),
            e(r"fn is_([A-Z]+)|fn as_([A-Z]+)"),
        );
        assert_eq!(
            inexact([I("foo")], [I("quux")]),
            e(r"foo[A-Z]+bar[A-Z]+quux")
        );
        assert_eq!(infinite(), e(r"[A-Z]+bar[A-Z]+"));
        assert_eq!(
            exact(["Sherlock Holmes"]),
            e(r"(?m)^Sherlock Holmes|Sherlock Holmes$")
        );

        assert_eq!(exact(["sa", "sb"]), e(r"\bs(?:[ab])"));
    }

    // This tests a specific regex along with some heuristic steps to reduce
    // the sequences extracted. This is meant to roughly correspond to the
    // types of heuristics used to shrink literal sets in practice. (Shrinking
    // is done because you want to balance "spend too much work looking for
    // too many literals" and "spend too much work processing false positive
    // matches from short literals.")
    #[test]
    #[cfg(feature = "unicode-case")]
    fn holmes() {
        let expected = inexact(
            ["HOL", "HOl", "HoL", "Hol", "hOL", "hOl", "hoL", "hol"].map(I),
            [
                "MES", "MEs", "Eſ", "MeS", "Mes", "eſ", "mES", "mEs", "meS",
                "mes",
            ]
            .map(I),
        );
        let (mut prefixes, mut suffixes) = e(r"(?i)Holmes");
        prefixes.keep_first_bytes(3);
        suffixes.keep_last_bytes(3);
        prefixes.minimize_by_preference();
        suffixes.minimize_by_preference();
        assert_eq!(expected, (prefixes, suffixes));
    }

    // This tests that we get some kind of literals extracted for a beefier
    // alternation with case insensitive mode enabled. At one point during
    // development, this returned nothing, and motivated some special case
    // code in Extractor::union to try and trim down the literal sequences
    // if the union would blow the limits set.
    #[test]
    #[cfg(feature = "unicode-case")]
    fn holmes_alt() {
        let mut pre =
            prefixes(r"(?i)Sherlock|Holmes|Watson|Irene|Adler|John|Baker");
        assert!(pre.len().unwrap() > 0);
        pre.optimize_for_prefix_by_preference();
        assert!(pre.len().unwrap() > 0);
    }

    // See: https://github.com/rust-lang/regex/security/advisories/GHSA-m5pq-gvj9-9vr8
    // See: CVE-2022-24713
    //
    // We test this here to ensure literal extraction completes in reasonable
    // time and isn't materially impacted by these sorts of pathological
    // repeats.
    #[test]
    fn crazy_repeats() {
        assert_eq!(inexact([E("")], [E("")]), e(r"(?:){4294967295}"));
        assert_eq!(
            inexact([E("")], [E("")]),
            e(r"(?:){64}{64}{64}{64}{64}{64}")
        );
        assert_eq!(inexact([E("")], [E("")]), e(r"x{0}{4294967295}"));
        assert_eq!(inexact([E("")], [E("")]), e(r"(?:|){4294967295}"));

        assert_eq!(
            inexact([E("")], [E("")]),
            e(r"(?:){8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}")
        );
        let repa = "a".repeat(100);
        assert_eq!(
            inexact([I(&repa)], [I(&repa)]),
            e(r"a{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}{8}")
        );
    }

    #[test]
    fn huge() {
        let pat = r#"(?-u)
        2(?:
          [45]\d{3}|
          7(?:
            1[0-267]|
            2[0-289]|
            3[0-29]|
            4[01]|
            5[1-3]|
            6[013]|
            7[0178]|
            91
          )|
          8(?:
            0[125]|
            [139][1-6]|
            2[0157-9]|
            41|
            6[1-35]|
            7[1-5]|
            8[1-8]|
            90
          )|
          9(?:
            0[0-2]|
            1[0-4]|
            2[568]|
            3[3-6]|
            5[5-7]|
            6[0167]|
            7[15]|
            8[0146-9]
          )
        )\d{4}|
        3(?:
          12?[5-7]\d{2}|
          0(?:
            2(?:
              [025-79]\d|
              [348]\d{1,2}
            )|
            3(?:
              [2-4]\d|
              [56]\d?
            )
          )|
          2(?:
            1\d{2}|
            2(?:
              [12]\d|
              [35]\d{1,2}|
              4\d?
            )
          )|
          3(?:
            1\d{2}|
            2(?:
              [2356]\d|
              4\d{1,2}
            )
          )|
          4(?:
            1\d{2}|
            2(?:
              2\d{1,2}|
              [47]|
              5\d{2}
            )
          )|
          5(?:
            1\d{2}|
            29
          )|
          [67]1\d{2}|
          8(?:
            1\d{2}|
            2(?:
              2\d{2}|
              3|
              4\d
            )
          )
        )\d{3}|
        4(?:
          0(?:
            2(?:
              [09]\d|
              7
            )|
            33\d{2}
          )|
          1\d{3}|
          2(?:
            1\d{2}|
            2(?:
              [25]\d?|
              [348]\d|
              [67]\d{1,2}
            )
          )|
          3(?:
            1\d{2}(?:
              \d{2}
            )?|
            2(?:
              [045]\d|
              [236-9]\d{1,2}
            )|
            32\d{2}
          )|
          4(?:
            [18]\d{2}|
            2(?:
              [2-46]\d{2}|
              3
            )|
            5[25]\d{2}
          )|
          5(?:
            1\d{2}|
            2(?:
              3\d|
              5
            )
          )|
          6(?:
            [18]\d{2}|
            2(?:
              3(?:
                \d{2}
              )?|
              [46]\d{1,2}|
              5\d{2}|
              7\d
            )|
            5(?:
              3\d?|
              4\d|
              [57]\d{1,2}|
              6\d{2}|
              8
            )
          )|
          71\d{2}|
          8(?:
            [18]\d{2}|
            23\d{2}|
            54\d{2}
          )|
          9(?:
            [18]\d{2}|
            2[2-5]\d{2}|
            53\d{1,2}
          )
        )\d{3}|
        5(?:
          02[03489]\d{2}|
          1\d{2}|
          2(?:
            1\d{2}|
            2(?:
              2(?:
                \d{2}
              )?|
              [457]\d{2}
            )
          )|
          3(?:
            1\d{2}|
            2(?:
              [37](?:
                \d{2}
              )?|
              [569]\d{2}
            )
          )|
          4(?:
            1\d{2}|
            2[46]\d{2}
          )|
          5(?:
            1\d{2}|
            26\d{1,2}
          )|
          6(?:
            [18]\d{2}|
            2|
            53\d{2}
          )|
          7(?:
            1|
            24
          )\d{2}|
          8(?:
            1|
            26
          )\d{2}|
          91\d{2}
        )\d{3}|
        6(?:
          0(?:
            1\d{2}|
            2(?:
              3\d{2}|
              4\d{1,2}
            )
          )|
          2(?:
            2[2-5]\d{2}|
            5(?:
              [3-5]\d{2}|
              7
            )|
            8\d{2}
          )|
          3(?:
            1|
            2[3478]
          )\d{2}|
          4(?:
            1|
            2[34]
          )\d{2}|
          5(?:
            1|
            2[47]
          )\d{2}|
          6(?:
            [18]\d{2}|
            6(?:
              2(?:
                2\d|
                [34]\d{2}
              )|
              5(?:
                [24]\d{2}|
                3\d|
                5\d{1,2}
              )
            )
          )|
          72[2-5]\d{2}|
          8(?:
            1\d{2}|
            2[2-5]\d{2}
          )|
          9(?:
            1\d{2}|
            2[2-6]\d{2}
          )
        )\d{3}|
        7(?:
          (?:
            02|
            [3-589]1|
            6[12]|
            72[24]
          )\d{2}|
          21\d{3}|
          32
        )\d{3}|
        8(?:
          (?:
            4[12]|
            [5-7]2|
            1\d?
          )|
          (?:
            0|
            3[12]|
            [5-7]1|
            217
          )\d
        )\d{4}|
        9(?:
          [35]1|
          (?:
            [024]2|
            81
          )\d|
          (?:
            1|
            [24]1
          )\d{2}
        )\d{3}
        "#;
        // TODO: This is a good candidate of a seq of literals that could be
        // shrunk quite a bit and still be very productive with respect to
        // literal optimizations.
        let (prefixes, suffixes) = e(pat);
        assert!(!suffixes.is_finite());
        assert_eq!(Some(243), prefixes.len());
    }

    #[test]
    fn optimize() {
        // This gets a common prefix that isn't too short.
        let (p, s) =
            opt(["foobarfoobar", "foobar", "foobarzfoobar", "foobarfoobar"]);
        assert_eq!(seq([I("foobar")]), p);
        assert_eq!(seq([I("foobar")]), s);

        // This also finds a common prefix, but since it's only one byte, it
        // prefers the multiple literals.
        let (p, s) = opt(["abba", "akka", "abccba"]);
        assert_eq!(exact(["abba", "akka", "abccba"]), (p, s));

        let (p, s) = opt(["sam", "samwise"]);
        assert_eq!((seq([E("sam")]), seq([E("sam"), E("samwise")])), (p, s));

        // The empty string is poisonous, so our seq becomes infinite, even
        // though all literals are exact.
        let (p, s) = opt(["foobarfoo", "foo", "", "foozfoo", "foofoo"]);
        assert!(!p.is_finite());
        assert!(!s.is_finite());

        // A space is also poisonous, so our seq becomes infinite. But this
        // only gets triggered when we don't have a completely exact sequence.
        // When the sequence is exact, spaces are okay, since we presume that
        // any prefilter will match a space more quickly than the regex engine.
        // (When the sequence is exact, there's a chance of the prefilter being
        // used without needing the regex engine at all.)
        let mut p = seq([E("foobarfoo"), I("foo"), E(" "), E("foofoo")]);
        p.optimize_for_prefix_by_preference();
        assert!(!p.is_finite());
    }
}
