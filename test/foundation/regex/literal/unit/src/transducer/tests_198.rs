#[cfg(test)]
mod tests {
    use alloc::{string::String, vec, vec::Vec};

    use fst::{Automaton, IntoStreamer, Set, Streamer};

    use crate::literal::{
        dfa::DFA,
        nfa::{contiguous, noncontiguous},
        StartKind,
    };

    use super::*;

    fn search<A: Automaton, D: AsRef<[u8]>>(
        set: &Set<D>,
        aut: A,
    ) -> Vec<String> {
        let mut stream = set.search(aut).into_stream();
        let mut results = vec![];
        while let Some(key) = stream.next() {
            results.push(String::from(core::str::from_utf8(key).unwrap()));
        }
        results
    }

    #[test]
    fn unanchored() {
        let set =
            Set::from_iter(&["a", "bar", "baz", "wat", "xba", "xbax", "z"])
                .unwrap();
        let patterns = vec!["baz", "bax"];
        let expected = vec!["baz", "xbax"];

        let aut = Unanchored(noncontiguous::NFA::new(&patterns).unwrap());
        let got = search(&set, &aut);
        assert_eq!(got, expected);

        let aut = Unanchored(contiguous::NFA::new(&patterns).unwrap());
        let got = search(&set, &aut);
        assert_eq!(got, expected);

        let aut = Unanchored(DFA::new(&patterns).unwrap());
        let got = search(&set, &aut);
        assert_eq!(got, expected);
    }

    #[test]
    fn anchored() {
        let set =
            Set::from_iter(&["a", "bar", "baz", "wat", "xba", "xbax", "z"])
                .unwrap();
        let patterns = vec!["baz", "bax"];
        let expected = vec!["baz"];

        let aut = Anchored(noncontiguous::NFA::new(&patterns).unwrap());
        let got = search(&set, &aut);
        assert_eq!(got, expected);

        let aut = Anchored(contiguous::NFA::new(&patterns).unwrap());
        let got = search(&set, &aut);
        assert_eq!(got, expected);

        let aut = Anchored(
            DFA::builder()
                .start_kind(StartKind::Anchored)
                .build(&patterns)
                .unwrap(),
        );
        let got = search(&set, &aut);
        assert_eq!(got, expected);
    }
}
