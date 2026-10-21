#[cfg(all(test, feature = "syntax"))]
mod tests {
    use super::*;

    // Tests that we handle heuristic Unicode word boundary support in reverse
    // DFAs in the specific case of contextual searches.
    //
    // I wrote this test when I discovered a bug in how heuristic word
    // boundaries were handled. Namely, that the starting state selection
    // didn't consider the DFA's quit byte set when looking at the byte
    // immediately before the start of the search (or immediately after the
    // end of the search in the case of a reverse search). As a result, it was
    // possible for '\bfoo\b' to match 'β123' because the trailing \xB2 byte
    // in the 'β' codepoint would be treated as a non-word character. But of
    // course, this search should trigger the DFA to quit, since there is a
    // non-ASCII byte in consideration.
    //
    // Thus, I fixed 'start_state_{forward,reverse}' to check the quit byte set
    // if it wasn't empty. The forward case is tested in the doc test for the
    // Config::unicode_word_boundary API. We test the reverse case here, which
    // is sufficiently niche that it doesn't really belong in a doc test.
    #[test]
    fn heuristic_unicode_reverse() {
        let dfa = DFA::builder()
            .configure(DFA::config().unicode_word_boundary(true))
            .thompson(thompson::Config::new().reverse(true))
            .build(r"\b[0-9]+\b")
            .unwrap();
        let mut cache = dfa.create_cache();

        let input = Input::new("β123").range(2..);
        let expected = MatchError::quit(0xB2, 1);
        let got = dfa.try_search_rev(&mut cache, &input);
        assert_eq!(Err(expected), got);

        let input = Input::new("123β").range(..3);
        let expected = MatchError::quit(0xCE, 3);
        let got = dfa.try_search_rev(&mut cache, &input);
        assert_eq!(Err(expected), got);
    }
}
