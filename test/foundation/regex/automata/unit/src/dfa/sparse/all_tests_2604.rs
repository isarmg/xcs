#[cfg(all(test, feature = "syntax", feature = "dfa-build"))]
mod tests {
    use crate::automata::{
        dfa::{dense::DFA, Automaton},
        nfa::thompson,
        Input, MatchError,
    };

    // See the analogous test in src/hybrid/dfa.rs and src/dfa/dense.rs.
    #[test]
    fn heuristic_unicode_forward() {
        let dfa = DFA::builder()
            .configure(DFA::config().unicode_word_boundary(true))
            .thompson(thompson::Config::new().reverse(true))
            .build(r"\b[0-9]+\b")
            .unwrap()
            .to_sparse()
            .unwrap();

        let input = Input::new("β123").range(2..);
        let expected = MatchError::quit(0xB2, 1);
        let got = dfa.try_search_fwd(&input);
        assert_eq!(Err(expected), got);

        let input = Input::new("123β").range(..3);
        let expected = MatchError::quit(0xCE, 3);
        let got = dfa.try_search_fwd(&input);
        assert_eq!(Err(expected), got);
    }

    // See the analogous test in src/hybrid/dfa.rs and src/dfa/dense.rs.
    #[test]
    fn heuristic_unicode_reverse() {
        let dfa = DFA::builder()
            .configure(DFA::config().unicode_word_boundary(true))
            .thompson(thompson::Config::new().reverse(true))
            .build(r"\b[0-9]+\b")
            .unwrap()
            .to_sparse()
            .unwrap();

        let input = Input::new("β123").range(2..);
        let expected = MatchError::quit(0xB2, 1);
        let got = dfa.try_search_rev(&input);
        assert_eq!(Err(expected), got);

        let input = Input::new("123β").range(..3);
        let expected = MatchError::quit(0xCE, 3);
        let got = dfa.try_search_rev(&input);
        assert_eq!(Err(expected), got);
    }

    // A starting state can never be a match state, since all matches are
    // delayed by one byte. The sparse DFA already rejects a serialized DFA
    // whose start table points at a match state, but only the dense check
    // had a regression test. See the test of the same name in
    // src/dfa/dense.rs.
    #[test]
    fn regression_start_state_not_match() {
        use crate::automata::util::primitives::StateID;

        let mut dfa = DFA::new("abc").unwrap().to_sparse().unwrap();
        let min_match = dfa.special.min_match.as_u32().to_ne_bytes();
        for entry in dfa.st.table.chunks_mut(StateID::SIZE) {
            entry.copy_from_slice(&min_match);
        }
        let buf = dfa.to_bytes_native_endian();
        crate::automata::dfa::sparse::DFA::from_bytes(&buf).unwrap_err();
    }
}
