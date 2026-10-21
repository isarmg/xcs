#[cfg(test)]
mod testoibits {
    use std::panic::{RefUnwindSafe, UnwindSafe};

    use super::*;

    fn assert_all<T: Send + Sync + UnwindSafe + RefUnwindSafe>() {}

    #[test]
    fn oibits_main() {
        assert_all::<AhoCorasick>();
        assert_all::<AhoCorasickBuilder>();
        assert_all::<AhoCorasickKind>();
        assert_all::<FindIter>();
        assert_all::<FindOverlappingIter>();

        assert_all::<BuildError>();
        assert_all::<MatchError>();
        assert_all::<MatchErrorKind>();

        assert_all::<Anchored>();
        assert_all::<Input>();
        assert_all::<Match>();
        assert_all::<MatchKind>();
        assert_all::<Span>();
        assert_all::<StartKind>();
    }

    #[test]
    fn oibits_automaton() {
        use crate::literal::{automaton, dfa::DFA};

        assert_all::<automaton::FindIter<DFA>>();
        assert_all::<automaton::FindOverlappingIter<DFA>>();
        #[cfg(feature = "std")]
        assert_all::<automaton::StreamFindIter<DFA, std::io::Stdin>>();
        assert_all::<automaton::OverlappingState>();

        assert_all::<automaton::Prefilter>();
        assert_all::<automaton::Candidate>();
    }

    #[test]
    fn oibits_packed() {
        use crate::literal::packed;

        assert_all::<packed::Config>();
        assert_all::<packed::Builder>();
        assert_all::<packed::Searcher>();
        assert_all::<packed::FindIter>();
        assert_all::<packed::MatchKind>();
    }
}
