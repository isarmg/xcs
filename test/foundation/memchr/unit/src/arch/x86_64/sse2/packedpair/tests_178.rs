#[cfg(test)]
mod tests {
    use super::*;

    fn find(haystack: &[u8], needle: &[u8]) -> Option<Option<usize>> {
        let f = Finder::new(needle)?;
        if haystack.len() < f.min_haystack_len() {
            return None;
        }
        Some(f.find(haystack, needle))
    }

    define_substring_forward_quickcheck!(find);

    #[test]
    fn forward_substring() {
        crate::tests::substring::Runner::new().fwd(find).run()
    }

    #[test]
    fn forward_packedpair() {
        fn find(
            haystack: &[u8],
            needle: &[u8],
            index1: u8,
            index2: u8,
        ) -> Option<Option<usize>> {
            let pair = Pair::with_indices(needle, index1, index2)?;
            let f = Finder::with_pair(needle, pair)?;
            if haystack.len() < f.min_haystack_len() {
                return None;
            }
            Some(f.find(haystack, needle))
        }
        crate::tests::packedpair::Runner::new().fwd(find).run()
    }

    #[test]
    fn forward_packedpair_prefilter() {
        fn find(
            haystack: &[u8],
            needle: &[u8],
            index1: u8,
            index2: u8,
        ) -> Option<Option<usize>> {
            let pair = Pair::with_indices(needle, index1, index2)?;
            let f = Finder::with_pair(needle, pair)?;
            if haystack.len() < f.min_haystack_len() {
                return None;
            }
            Some(f.find_prefilter(haystack))
        }
        crate::tests::packedpair::Runner::new().fwd(find).run()
    }

    // A demonstration of UB (caught under Miri) through some of memchr's
    // lower level but public APIs.
    //
    // See: https://github.com/BurntSushi/memchr/issues/225
    #[test]
    fn regression_undefined_behavior() {
        let construct_needle: &[u8] = b"ab"; // length 2 → pair indices 0,1
        let pair = Pair::new(construct_needle).unwrap();
        let Some(finder) = Finder::with_pair(construct_needle, pair) else {
            return;
        };
        let mhl = finder.min_haystack_len(); // 17
        let mut haystack = alloc::vec![b'.'; mhl + 5];
        haystack[0] = b'a';
        // force entry into find_in_chunk inner branch
        haystack[1] = b'b';
        // 122 > haystack.len()=22
        let long_needle = alloc::vec![b'X'; haystack.len() + 100];
        let _ = finder.find(&haystack, &long_needle);
    }
}
