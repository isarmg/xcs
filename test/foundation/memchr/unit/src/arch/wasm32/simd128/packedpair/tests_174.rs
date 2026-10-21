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
}
