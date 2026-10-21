#[cfg(test)]
mod tests {
    use core::ops::RangeInclusive;
    use std::collections::HashSet;

    use super::*;

    // "aabxzz" => [a..=a, b..=x, z..=z]
    fn range_iter_from_str(s: &str) -> impl Iterator<Item = RangeInclusive<u32>> {
        debug_assert_eq!(
            s.chars().count() % 2,
            0,
            "string \"{}\" does not contain an even number of code points",
            s.escape_debug()
        );
        let mut res = vec![];
        let mut skip = false;
        for (a, b) in s.chars().zip(s.chars().skip(1)) {
            if skip {
                skip = false;
                continue;
            }
            let a = a as u32;
            let b = b as u32;
            res.push(a..=b);
            skip = true;
        }

        res.into_iter()
    }

    fn assert_set_equality<'a>(
        source: &str,
        cpinvlistandstrlist: &CodePointInversionListAndStringList,
        single: impl Iterator<Item = RangeInclusive<u32>>,
        strings: impl Iterator<Item = &'a str>,
    ) {
        let expected_ranges: HashSet<_> = single.collect();
        let actual_ranges: HashSet<_> = cpinvlistandstrlist.code_points().iter_ranges().collect();
        assert_eq!(
            actual_ranges,
            expected_ranges,
            "got unexpected ranges {:?}, expected {:?} for parsed set \"{}\"",
            actual_ranges,
            expected_ranges,
            source.escape_debug()
        );
        let mut expected_size = cpinvlistandstrlist.code_points().size();
        for s in strings {
            expected_size += 1;
            assert!(
                cpinvlistandstrlist.contains_str(s),
                "missing string \"{}\" from parsed set \"{}\"",
                s.escape_debug(),
                source.escape_debug()
            );
        }
        let actual_size = cpinvlistandstrlist.size();
        assert_eq!(
            actual_size,
            expected_size,
            "got unexpected size {}, expected {} for parsed set \"{}\"",
            actual_size,
            expected_size,
            source.escape_debug()
        );
    }

    fn assert_is_error_and_message_eq(source: &str, expected_err: &str, vm: &VariableMap<'_>) {
        let result = parse_with_variables(source, vm);
        assert!(result.is_err(), "{source} does not cause an error!");
        let err = result.unwrap_err();
        assert_eq!(err.fmt_with_source(source).to_string(), expected_err);
    }

    #[test]
    fn test_semantics_with_variables() {
        let mut map_char_char = VariableMap::default();
        map_char_char.insert_char("a".to_string(), 'a').unwrap();
        map_char_char.insert_char("var2".to_string(), 'z').unwrap();

        let mut map_headache = VariableMap::default();
        map_headache.insert_char("hehe".to_string(), '-').unwrap();

        let mut map_char_string = VariableMap::default();
        map_char_string.insert_char("a".to_string(), 'a').unwrap();
        map_char_string
            .insert_string("var2".to_string(), "abc".to_string())
            .unwrap();

        let (set, _) = parse(r"[a-z {Hello,\ World!}]").unwrap();
        let mut map_char_set = VariableMap::default();
        map_char_set.insert_char("a".to_string(), 'a').unwrap();
        map_char_set.insert_set("set".to_string(), set).unwrap();

        let cases: Vec<(_, _, _, Vec<&str>)> = vec![
            // simple
            (&map_char_char, "[$a]", "aa", vec![]),
            (&map_char_char, "[ $a ]", "aa", vec![]),
            (&map_char_char, "[$a$]", "aa\u{ffff}\u{ffff}", vec![]),
            (&map_char_char, "[$a$ ]", "aa\u{ffff}\u{ffff}", vec![]),
            (&map_char_char, "[$a$var2]", "aazz", vec![]),
            (&map_char_char, "[$a - $var2]", "az", vec![]),
            (&map_char_char, "[$a-$var2]", "az", vec![]),
            (&map_headache, "[a $hehe z]", "aazz--", vec![]),
            (
                &map_char_char,
                "[[$]var2]",
                "\u{ffff}\u{ffff}vvaarr22",
                vec![],
            ),
            // variable prefix escaping
            (&map_char_char, r"[\$var2]", "$$vvaarr22", vec![]),
            (&map_char_char, r"[\\$var2]", r"\\zz", vec![]),
            // no variable dereferencing in strings
            (&map_char_char, "[{$a}]", "", vec!["$a"]),
            // set operations
            (&map_char_set, "[$set & [b-z]]", "bz", vec![]),
            (&map_char_set, "[[a-z]-[b-z]]", "aa", vec![]),
            (&map_char_set, "[$set-[b-z]]", "aa", vec!["Hello, World!"]),
            (&map_char_set, "[$set-$set]", "", vec![]),
            (&map_char_set, "[[a-zA]-$set]", "AA", vec![]),
            (&map_char_set, "[$set[b-z]]", "az", vec!["Hello, World!"]),
            (&map_char_set, "[[a-a]$set]", "az", vec!["Hello, World!"]),
            (&map_char_set, "$set", "az", vec!["Hello, World!"]),
            // strings
            (&map_char_string, "[$var2]", "", vec!["abc"]),
        ];
        for (variable_map, source, single, strings) in cases {
            let parsed = parse_with_variables(source, variable_map);
            if let Err(err) = parsed {
                panic!(
                    "{source} results in an error: {}",
                    err.fmt_with_source(source)
                );
            }
            let (set, consumed) = parsed.unwrap();
            assert_eq!(consumed, source.len(), "{source:?} is not fully consumed");
            assert_set_equality(
                source,
                &set,
                range_iter_from_str(single),
                strings.into_iter(),
            );
        }
    }

    #[test]
    fn test_semantics() {
        const ALL_CHARS: &str = "\x00\u{10FFFF}";
        let cases: Vec<(_, _, Vec<&str>)> = vec![
            // simple
            ("[a]", "aa", vec![]),
            ("[]", "", vec![]),
            ("[qax]", "aaqqxx", vec![]),
            ("[a-z]", "az", vec![]),
            ("[--]", "--", vec![]),
            ("[a-b-]", "ab--", vec![]),
            ("[[a-b]-]", "ab--", vec![]),
            ("[{ab}-]", "--", vec!["ab"]),
            ("[-a-b]", "ab--", vec![]),
            ("[-a]", "--aa", vec![]),
            // whitespace escaping
            (r"[\n]", "\n\n", vec![]),
            ("[\\\n]", "\n\n", vec![]),
            // empty - whitespace is skipped
            ("[\n]", "", vec![]),
            ("[\u{9}]", "", vec![]),
            ("[\u{A}]", "", vec![]),
            ("[\u{B}]", "", vec![]),
            ("[\u{C}]", "", vec![]),
            ("[\u{D}]", "", vec![]),
            ("[\u{20}]", "", vec![]),
            ("[\u{85}]", "", vec![]),
            ("[\u{200E}]", "", vec![]),
            ("[\u{200F}]", "", vec![]),
            ("[\u{2028}]", "", vec![]),
            ("[\u{2029}]", "", vec![]),
            // whitespace significance:
            ("[^[^$]]", "\u{ffff}\u{ffff}", vec![]),
            ("[^[^ $]]", "\u{ffff}\u{ffff}", vec![]),
            ("[^[^ $ ]]", "\u{ffff}\u{ffff}", vec![]),
            ("[^[^a$]]", "aa\u{ffff}\u{ffff}", vec![]),
            ("[^[^a$ ]]", "aa\u{ffff}\u{ffff}", vec![]),
            ("[-]", "--", vec![]),
            ("[  -  ]", "--", vec![]),
            ("[  - -  ]", "--", vec![]),
            ("[ a-b -  ]", "ab--", vec![]),
            ("[ -a]", "--aa", vec![]),
            ("[a-]", "--aa", vec![]),
            ("[a- ]", "--aa", vec![]),
            ("[ :]", "::", vec![]),
            ("[ :L:]", "::LL", vec![]),
            // but not all "whitespace", only Pattern_White_Space:
            ("[\u{A0}]", "\u{A0}\u{A0}", vec![]), // non-breaking space
            // anchor
            ("[$]", "\u{ffff}\u{ffff}", vec![]),
            (r"[\$]", "$$", vec![]),
            ("[{$}]", "$$", vec![]),
            // set operations
            ("[[a-z]&[b-z]]", "bz", vec![]),
            ("[[a-z]-[b-z]]", "aa", vec![]),
            ("[[a-z][b-z]]", "az", vec![]),
            ("[[a-a][b-z]]", "az", vec![]),
            ("[[a-z{abc}]&[b-z{abc}{abx}]]", "bz", vec!["abc"]),
            ("[[{abx}a-z{abc}]&[b-z{abc}]]", "bz", vec!["abc"]),
            ("[[a-z{abx}]-[{abx}b-z{abc}]]", "aa", vec![]),
            ("[[a-z{abx}{abc}]-[{abx}b-z]]", "aa", vec!["abc"]),
            ("[[a-z{abc}][b-z{abx}]]", "az", vec!["abc", "abx"]),
            // strings
            ("[{this is a minus -}]", "", vec!["thisisaminus-"]),
            // associativity
            ("[[a-a][b-z] - [a-d][e-z]]", "ez", vec![]),
            ("[[a-a][b-z] - [a-d]&[e-z]]", "ez", vec![]),
            ("[[a-a][b-z] - [a-z][]]", "", vec![]),
            ("[[a-a][b-z] - [a-z]&[]]", "", vec![]),
            ("[[a-a][b-z] & [a-z]-[]]", "az", vec![]),
            ("[[a-a][b-z] & []-[a-z]]", "", vec![]),
            ("[[a-a][b-z] & [a-b][x-z]]", "abxz", vec![]),
            ("[[a-z]-[a-b]-[y-z]]", "cx", vec![]),
            // escape tests
            (r"[\x61-\x63]", "ac", vec![]),
            (r"[a-\x63]", "ac", vec![]),
            (r"[\x61-c]", "ac", vec![]),
            (r"[\u0061-\x63]", "ac", vec![]),
            (r"[\U00000061-\x63]", "ac", vec![]),
            (r"[\x{61}-\x63]", "ac", vec![]),
            (r"[\u{61}-\x63]", "ac", vec![]),
            (r"[\u{61}{hello\ world}]", "aa", vec!["hello world"]),
            (r"[{hello\ world}\u{61}]", "aa", vec!["hello world"]),
            (r"[{h\u{65}llo\ world}]", "", vec!["hello world"]),
            // complement tests
            (r"[^]", ALL_CHARS, vec![]),
            (r"[[^]-[^a-z]]", "az", vec![]),
            (r"[^{h\u{65}llo\ world}]", ALL_CHARS, vec![]),
            (
                r"[^[{h\u{65}llo\ world}]-[{hello\ world}]]",
                ALL_CHARS,
                vec![],
            ),
            (
                r"[^[\x00-\U0010FFFF]-[\u0100-\U0010FFFF]]",
                "\u{100}\u{10FFFF}",
                vec![],
            ),
            (r"[^[^a-z]]", "az", vec![]),
            (r"[^[^\^]]", "^^", vec![]),
            (r"[{\x{61 0062   063}}]", "", vec!["abc"]),
            (r"[\x{61 0062   063}]", "ac", vec![]),
            // binary properties
            (r"[:AHex:]", "09afAF", vec![]),
            (r"[:AHex=True:]", "09afAF", vec![]),
            (r"[:AHex=T:]", "09afAF", vec![]),
            (r"[:AHex=Yes:]", "09afAF", vec![]),
            (r"[:AHex=Y:]", "09afAF", vec![]),
            (r"[:^AHex≠True:]", "09afAF", vec![]),
            (r"[:AHex≠False:]", "09afAF", vec![]),
            (r"[[:^AHex≠False:]&[\x00-\x10]]", "\0\x10", vec![]),
            (r"\p{AHex}", "09afAF", vec![]),
            (r"\p{AHex=True}", "09afAF", vec![]),
            (r"\p{AHex=T}", "09afAF", vec![]),
            (r"\p{AHex=Yes}", "09afAF", vec![]),
            (r"\p{AHex=Y}", "09afAF", vec![]),
            (r"\P{AHex≠True}", "09afAF", vec![]),
            (r"\p{AHex≠False}", "09afAF", vec![]),
            // general category
            (r"[[:gc=lower-case-letter:]&[a-zA-Z]]", "az", vec![]),
            (r"[[:lower case letter:]&[a-zA-Z]]", "az", vec![]),
            // general category groups
            // equivalence between L and the union of all the L* categories
            (
                r"[[[:L:]-[\p{Ll}\p{Lt}\p{Lu}\p{Lo}\p{Lm}]][[\p{Ll}\p{Lt}\p{Lu}\p{Lo}\p{Lm}]-[:L:]]]",
                "",
                vec![],
            ),
            // script
            (r"[[:sc=latn:]&[a-zA-Z]]", "azAZ", vec![]),
            (r"[[:sc=Latin:]&[a-zA-Z]]", "azAZ", vec![]),
            (r"[[:Latin:]&[a-zA-Z]]", "azAZ", vec![]),
            (r"[[:latn:]&[a-zA-Z]]", "azAZ", vec![]),
            // script extensions
            (r"[[:scx=latn:]&[a-zA-Z]]", "azAZ", vec![]),
            (r"[[:scx=Latin:]&[a-zA-Z]]", "azAZ", vec![]),
            (r"[[:scx=Hira:]&[\u30FC]]", "\u{30FC}\u{30FC}", vec![]),
            (r"[[:sc=Hira:]&[\u30FC]]", "", vec![]),
            (r"[[:scx=Kana:]&[\u30FC]]", "\u{30FC}\u{30FC}", vec![]),
            (r"[[:sc=Kana:]&[\u30FC]]", "", vec![]),
            (r"[[:sc=Common:]&[\u30FC]]", "\u{30FC}\u{30FC}", vec![]),
            // grapheme cluster break
            (
                r"\p{Grapheme_Cluster_Break=ZWJ}",
                "\u{200D}\u{200D}",
                vec![],
            ),
            // sentence break
            (
                r"\p{Sentence_Break=ATerm}",
                "\u{002E}\u{002E}\u{2024}\u{2024}\u{FE52}\u{FE52}\u{FF0E}\u{FF0E}",
                vec![],
            ),
            // word break
            (r"\p{Word_Break=Single_Quote}", "\u{0027}\u{0027}", vec![]),
            // more syntax edge cases from UTS35 directly
            (r"[\^a]", "^^aa", vec![]),
            (r"[{{}]", "{{", vec![]),
            (r"[{}}]", "}}", vec![""]),
            (r"[}]", "}}", vec![]),
            (r"[{$var}]", "", vec!["$var"]),
            (r"[{[a-z}]", "", vec!["[a-z"]),
            (r"[ { [ a - z } ]", "", vec!["[a-z"]),
            // TODO(#3556): Add more tests (specifically conformance tests if they exist)
        ];
        for (source, single, strings) in cases {
            let parsed = parse(source);
            if let Err(err) = parsed {
                panic!(
                    "{source} results in an error: {}",
                    err.fmt_with_source(source)
                );
            }
            let (set, consumed) = parsed.unwrap();
            assert_eq!(consumed, source.len());
            assert_set_equality(
                source,
                &set,
                range_iter_from_str(single),
                strings.into_iter(),
            );
        }
    }

    #[test]
    fn test_error_messages_with_variables() {
        let mut map_char_char = VariableMap::default();
        map_char_char.insert_char("a".to_string(), 'a').unwrap();
        map_char_char.insert_char("var2".to_string(), 'z').unwrap();

        let mut map_char_string = VariableMap::default();
        map_char_string.insert_char("a".to_string(), 'a').unwrap();
        map_char_string
            .insert_string("var2".to_string(), "abc".to_string())
            .unwrap();

        let (set, _) = parse(r"[a-z {Hello,\ World!}]").unwrap();
        let mut map_char_set = VariableMap::default();
        map_char_set.insert_char("a".to_string(), 'a').unwrap();
        map_char_set.insert_set("set".to_string(), set).unwrap();

        let cases = [
            (&map_char_char, "[$$a]", r"[$$a← error: unexpected variable"),
            (
                &map_char_char,
                "[$ a]",
                r"[$ a← error: unexpected character 'a'",
            ),
            (&map_char_char, "$a", r"$a← error: unexpected variable"),
            (&map_char_char, "$", r"$← error: unexpected end of input"),
            (
                &map_char_string,
                "[$var2-$a]",
                r"[$var2-$a← error: unexpected variable",
            ),
            (
                &map_char_string,
                "[$a-$var2]",
                r"[$a-$var2← error: unexpected variable",
            ),
            (
                &map_char_set,
                "[$a-$set]",
                r"[$a-$set← error: unexpected variable",
            ),
            (
                &map_char_set,
                "[$set-$a]",
                r"[$set-$a← error: unexpected variable",
            ),
            (
                &map_char_set,
                "[$=]",
                "[$=← error: unexpected character '='",
            ),
        ];
        for (variable_map, source, expected_err) in cases {
            assert_is_error_and_message_eq(source, expected_err, variable_map);
        }
    }

    #[test]
    fn test_error_messages() {
        let cases = [
            (r"[a-z[\]]", r"[a-z[\]]← error: unexpected end of input"),
            (r"", r"← error: unexpected end of input"),
            (r"[{]", r"[{]← error: unexpected end of input"),
            // we match ECMA-262 strictly, so case matters
            (
                r"[:general_category:]",
                r"[:general_category← error: unknown property",
            ),
            (r"[:ll=true:]", r"[:ll=true← error: unknown property"),
            (r"[:=", r"[:=← error: unexpected character '='"),
            // property names may not be empty
            (r"[::]", r"[::← error: unexpected character ':'"),
            (r"[:=hello:]", r"[:=← error: unexpected character '='"),
            // property values may not be empty
            (r"[:gc=:]", r"[:gc=:← error: unexpected character ':'"),
            (r"[\xag]", r"[\xag← error: unexpected character 'g'"),
            (r"[a-b-z]", r"[a-b-z← error: unexpected character 'z'"),
            // TODO(#3558): Might be better as "[a-\p← error: unexpected character 'p'"?
            (r"[a-\p{ll}]", r"[a-\← error: unexpected character '\\'"),
            (r"[a-&]", r"[a-&← error: unexpected character '&'"),
            (r"[a&b]", r"[a&← error: unexpected character '&'"),
            (r"[[set]&b]", r"[[set]&b← error: unexpected character 'b'"),
            (r"[[set]&]", r"[[set]&]← error: unexpected character ']'"),
            (r"[a-\x60]", r"[a-\x60← error: unexpected character '`'"),
            (r"[a-`]", r"[a-`← error: unexpected character '`'"),
            (r"[\x{6g}]", r"[\x{6g← error: unexpected character 'g'"),
            (r"[\x{g}]", r"[\x{g← error: unexpected character 'g'"),
            (r"[\x{}]", r"[\x{}← error: unexpected character '}'"),
            (
                r"[\x{dabeef}]",
                r"[\x{dabeef← error: invalid escape sequence",
            ),
            (
                r"[\x{10ffff0}]",
                r"[\x{10ffff0← error: unexpected character '0'",
            ),
            (
                r"[\x{11ffff}]",
                r"[\x{11ffff← error: invalid escape sequence",
            ),
            (
                r"[\x{10ffff 1 10ffff0}]",
                r"[\x{10ffff 1 10ffff0← error: unexpected character '0'",
            ),
            // > 1 byte in UTF-8 edge case
            (r"ä", r"ä← error: unexpected character 'ä'"),
            (r"\p{gc=ä}", r"\p{gc=ä← error: unknown property"),
            (r"\p{gc=ä}", r"\p{gc=ä← error: unknown property"),
            (
                r"[\xe5-\xe4]",
                r"[\xe5-\xe4← error: unexpected character 'ä'",
            ),
            (r"[\xe5-ä]", r"[\xe5-ä← error: unexpected character 'ä'"),
            // whitespace significance
            (r"[ ^]", r"[ ^← error: unexpected character '^'"),
            (r"[:]", r"[:]← error: unexpected character ']'"),
            (r"[:L]", r"[:L]← error: unexpected character ']'"),
            (r"\p {L}", r"\p ← error: unexpected character ' '"),
            // multi-escapes are not allowed in ranges
            (
                r"[\x{61 62}-d]",
                r"[\x{61 62}-d← error: unexpected character 'd'",
            ),
            (
                r"[\x{61 63}-\x{62 64}]",
                r"[\x{61 63}-\← error: unexpected character '\\'",
            ),
            // TODO(#3558): This is a bad error message.
            (r"[a-\x{62 64}]", r"[a-\← error: unexpected character '\\'"),
        ];
        let vm = Default::default();
        for (source, expected_err) in cases {
            assert_is_error_and_message_eq(source, expected_err, &vm);
        }
    }

    #[test]
    fn test_consumed() {
        let cases = [
            (r"[a-z\]{[}]".len(), r"[a-z\]{[}][]"),
            (r"[a-z\]{[}]".len(), r"[a-z\]{[}] []"),
            (r"[a-z\]{[}]".len(), r"[a-z\]{]}] []"),
            (r"[a-z\]{{[}]".len(), r"[a-z\]{{]}] []"),
            (r"[a-z\]{[}]".len(), r"[a-z\]{]}]\p{L}"),
            (r"[a-z\]{[}]".len(), r"[a-z\]{]}]$var"),
        ];

        let vm = Default::default();
        for (expected_consumed, source) in cases {
            let (_, consumed) = parse(source).unwrap();
            assert_eq!(expected_consumed, consumed);
            let (_, consumed) = parse_with_variables(source, &vm).unwrap();
            assert_eq!(expected_consumed, consumed);
        }
    }
}
