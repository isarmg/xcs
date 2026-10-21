#[cfg(all(test, feature = "syntax", feature = "dfa-build"))]
mod tests {
    use crate::{Input, MatchError};

    use super::*;

    #[test]
    fn errors_with_unicode_word_boundary() {
        let pattern = r"\b";
        assert!(Builder::new().build(pattern).is_err());
    }

    #[test]
    fn roundtrip_never_match() {
        let dfa = DFA::never_match().unwrap();
        let (buf, _) = dfa.to_bytes_native_endian();
        let dfa: DFA<&[u32]> = DFA::from_bytes(&buf).unwrap().0;

        assert_eq!(None, dfa.try_search_fwd(&Input::new("foo12345")).unwrap());
    }

    #[test]
    fn roundtrip_always_match() {
        use crate::HalfMatch;

        let dfa = DFA::always_match().unwrap();
        let (buf, _) = dfa.to_bytes_native_endian();
        let dfa: DFA<&[u32]> = DFA::from_bytes(&buf).unwrap().0;

        assert_eq!(
            Some(HalfMatch::must(0, 0)),
            dfa.try_search_fwd(&Input::new("foo12345")).unwrap()
        );
    }

    // See the analogous test in src/hybrid/dfa.rs.
    #[test]
    fn heuristic_unicode_reverse() {
        let dfa = DFA::builder()
            .configure(DFA::config().unicode_word_boundary(true))
            .thompson(thompson::Config::new().reverse(true))
            .build(r"\b[0-9]+\b")
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

    // This panics in `TransitionTable::validate` if the match states are not
    // validated first.
    //
    // See: https://github.com/rust-lang/regex/pull/1295
    #[test]
    fn regression_validation_order() {
        let mut dfa = DFA::new("abc").unwrap();
        dfa.ms = MatchStates {
            slices: vec![],
            pattern_ids: vec![],
            pattern_len: 1,
        };
        let (buf, _) = dfa.to_bytes_native_endian();
        DFA::from_bytes(&buf).unwrap_err();
    }

    // A starting state can never be a match state, since all matches are
    // delayed by one byte. The search routines rely on this and assert it,
    // so `from_bytes` must reject a serialized DFA whose start table points
    // at a match state. The sparse DFA already rejected this; the dense DFA
    // did not, so searching with such a DFA tripped the assertion in
    // `dfa::search::init_fwd`.
    #[test]
    fn regression_start_state_not_match() {
        let mut dfa = DFA::new("abc").unwrap();
        let min_match = dfa.special.min_match;
        for id in dfa.st.table_mut() {
            *id = min_match;
        }
        let (buf, _) = dfa.to_bytes_native_endian();
        DFA::from_bytes(&buf).unwrap_err();
    }
}
