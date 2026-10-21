#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_packedpair() {
        fn find(
            haystack: &[u8],
            needle: &[u8],
            _index1: u8,
            _index2: u8,
        ) -> Option<Option<usize>> {
            // We ignore the index positions requested since it winds up making
            // this test too slow overall.
            let f = Finder::new(needle)?;
            Some(f.find_prefilter(haystack))
        }
        crate::tests::packedpair::Runner::new().fwd(find).run()
    }
}
