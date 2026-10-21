#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    /// Convenience wrapper for computing the suffix as a byte string.
    fn get_suffix_forward(needle: &[u8], kind: SuffixKind) -> (&[u8], usize) {
        let s = Suffix::forward(needle, kind);
        (&needle[s.pos..], s.period)
    }

    /// Convenience wrapper for computing the reverse suffix as a byte string.
    fn get_suffix_reverse(needle: &[u8], kind: SuffixKind) -> (&[u8], usize) {
        let s = Suffix::reverse(needle, kind);
        (&needle[..s.pos], s.period)
    }

    /// Return all of the non-empty suffixes in the given byte string.
    fn suffixes(bytes: &[u8]) -> Vec<&[u8]> {
        (0..bytes.len()).map(|i| &bytes[i..]).collect()
    }

    /// Return the lexicographically maximal suffix of the given byte string.
    fn naive_maximal_suffix_forward(needle: &[u8]) -> &[u8] {
        let mut sufs = suffixes(needle);
        sufs.sort();
        sufs.pop().unwrap()
    }

    /// Return the lexicographically maximal suffix of the reverse of the given
    /// byte string.
    fn naive_maximal_suffix_reverse(needle: &[u8]) -> Vec<u8> {
        let mut reversed = needle.to_vec();
        reversed.reverse();
        let mut got = naive_maximal_suffix_forward(&reversed).to_vec();
        got.reverse();
        got
    }

    define_substring_forward_quickcheck!(|h, n| Some(
        Finder::new(n).find(h, n)
    ));
    define_substring_reverse_quickcheck!(|h, n| Some(
        FinderRev::new(n).rfind(h, n)
    ));

    #[test]
    fn forward() {
        crate::tests::substring::Runner::new()
            .fwd(|h, n| Some(Finder::new(n).find(h, n)))
            .run();
    }

    #[test]
    fn reverse() {
        crate::tests::substring::Runner::new()
            .rev(|h, n| Some(FinderRev::new(n).rfind(h, n)))
            .run();
    }

    #[test]
    fn suffix_forward() {
        macro_rules! assert_suffix_min {
            ($given:expr, $expected:expr, $period:expr) => {
                let (got_suffix, got_period) =
                    get_suffix_forward($given.as_bytes(), SuffixKind::Minimal);
                let got_suffix = core::str::from_utf8(got_suffix).unwrap();
                assert_eq!(($expected, $period), (got_suffix, got_period));
            };
        }

        macro_rules! assert_suffix_max {
            ($given:expr, $expected:expr, $period:expr) => {
                let (got_suffix, got_period) =
                    get_suffix_forward($given.as_bytes(), SuffixKind::Maximal);
                let got_suffix = core::str::from_utf8(got_suffix).unwrap();
                assert_eq!(($expected, $period), (got_suffix, got_period));
            };
        }

        assert_suffix_min!("a", "a", 1);
        assert_suffix_max!("a", "a", 1);

        assert_suffix_min!("ab", "ab", 2);
        assert_suffix_max!("ab", "b", 1);

        assert_suffix_min!("ba", "a", 1);
        assert_suffix_max!("ba", "ba", 2);

        assert_suffix_min!("abc", "abc", 3);
        assert_suffix_max!("abc", "c", 1);

        assert_suffix_min!("acb", "acb", 3);
        assert_suffix_max!("acb", "cb", 2);

        assert_suffix_min!("cba", "a", 1);
        assert_suffix_max!("cba", "cba", 3);

        assert_suffix_min!("abcabc", "abcabc", 3);
        assert_suffix_max!("abcabc", "cabc", 3);

        assert_suffix_min!("abcabcabc", "abcabcabc", 3);
        assert_suffix_max!("abcabcabc", "cabcabc", 3);

        assert_suffix_min!("abczz", "abczz", 5);
        assert_suffix_max!("abczz", "zz", 1);

        assert_suffix_min!("zzabc", "abc", 3);
        assert_suffix_max!("zzabc", "zzabc", 5);

        assert_suffix_min!("aaa", "aaa", 1);
        assert_suffix_max!("aaa", "aaa", 1);

        assert_suffix_min!("foobar", "ar", 2);
        assert_suffix_max!("foobar", "r", 1);
    }

    #[test]
    fn suffix_reverse() {
        macro_rules! assert_suffix_min {
            ($given:expr, $expected:expr, $period:expr) => {
                let (got_suffix, got_period) =
                    get_suffix_reverse($given.as_bytes(), SuffixKind::Minimal);
                let got_suffix = core::str::from_utf8(got_suffix).unwrap();
                assert_eq!(($expected, $period), (got_suffix, got_period));
            };
        }

        macro_rules! assert_suffix_max {
            ($given:expr, $expected:expr, $period:expr) => {
                let (got_suffix, got_period) =
                    get_suffix_reverse($given.as_bytes(), SuffixKind::Maximal);
                let got_suffix = core::str::from_utf8(got_suffix).unwrap();
                assert_eq!(($expected, $period), (got_suffix, got_period));
            };
        }

        assert_suffix_min!("a", "a", 1);
        assert_suffix_max!("a", "a", 1);

        assert_suffix_min!("ab", "a", 1);
        assert_suffix_max!("ab", "ab", 2);

        assert_suffix_min!("ba", "ba", 2);
        assert_suffix_max!("ba", "b", 1);

        assert_suffix_min!("abc", "a", 1);
        assert_suffix_max!("abc", "abc", 3);

        assert_suffix_min!("acb", "a", 1);
        assert_suffix_max!("acb", "ac", 2);

        assert_suffix_min!("cba", "cba", 3);
        assert_suffix_max!("cba", "c", 1);

        assert_suffix_min!("abcabc", "abca", 3);
        assert_suffix_max!("abcabc", "abcabc", 3);

        assert_suffix_min!("abcabcabc", "abcabca", 3);
        assert_suffix_max!("abcabcabc", "abcabcabc", 3);

        assert_suffix_min!("abczz", "a", 1);
        assert_suffix_max!("abczz", "abczz", 5);

        assert_suffix_min!("zzabc", "zza", 3);
        assert_suffix_max!("zzabc", "zz", 1);

        assert_suffix_min!("aaa", "aaa", 1);
        assert_suffix_max!("aaa", "aaa", 1);
    }

    #[cfg(not(miri))]
    quickcheck::quickcheck! {
        fn qc_suffix_forward_maximal(bytes: Vec<u8>) -> bool {
            if bytes.is_empty() {
                return true;
            }

            let (got, _) = get_suffix_forward(&bytes, SuffixKind::Maximal);
            let expected = naive_maximal_suffix_forward(&bytes);
            got == expected
        }

        fn qc_suffix_reverse_maximal(bytes: Vec<u8>) -> bool {
            if bytes.is_empty() {
                return true;
            }

            let (got, _) = get_suffix_reverse(&bytes, SuffixKind::Maximal);
            let expected = naive_maximal_suffix_reverse(&bytes);
            expected == got
        }
    }

    // This is a regression test caught by quickcheck that exercised a bug in
    // the reverse small period handling. The bug was that we were using 'if j
    // == shift' to determine if a match occurred, but the correct guard is 'if
    // j >= shift', which matches the corresponding guard in the forward impl.
    #[test]
    fn regression_rev_small_period() {
        let rfind = |h, n| FinderRev::new(n).rfind(h, n);
        let haystack = "ababaz";
        let needle = "abab";
        assert_eq!(Some(0), rfind(haystack.as_bytes(), needle.as_bytes()));
    }
}
