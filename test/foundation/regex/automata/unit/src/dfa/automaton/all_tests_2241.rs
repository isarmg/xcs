#[cfg(all(test, feature = "syntax", feature = "dfa-build"))]
mod tests {
    // A basic test ensuring that our Automaton trait is object safe. (This is
    // the main reason why we don't define the search routines as generic over
    // Into<Input>.)
    #[test]
    fn object_safe() {
        use crate::automata::{
            dfa::{dense, Automaton},
            HalfMatch, Input,
        };

        let dfa = dense::DFA::new("abc").unwrap();
        let dfa: &dyn Automaton = &dfa;
        assert_eq!(
            Ok(Some(HalfMatch::must(0, 6))),
            dfa.try_search_fwd(&Input::new(b"xyzabcxyz")),
        );
    }
}
