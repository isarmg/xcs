#[cfg(all(test, feature = "syntax"))]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn fail_conflicting_transition() {
        let predicate = |err: &str| err.contains("conflicting transition");

        let err = DFA::new(r"a*[ab]").unwrap_err().to_string();
        assert!(predicate(&err), "{err}");
    }

    #[test]
    fn fail_multiple_epsilon() {
        let predicate = |err: &str| {
            err.contains("multiple epsilon transitions to same state")
        };

        let err = DFA::new(r"(^|$)a").unwrap_err().to_string();
        assert!(predicate(&err), "{err}");
    }

    #[test]
    fn fail_multiple_match() {
        let predicate = |err: &str| {
            err.contains("multiple epsilon transitions to match state")
        };

        let err = DFA::new_many(&[r"^", r"$"]).unwrap_err().to_string();
        assert!(predicate(&err), "{err}");
    }

    // This test is meant to build a one-pass regex with the maximum number of
    // possible slots.
    //
    // NOTE: Remember that the slot limit only applies to explicit capturing
    // groups. Any number of implicit capturing groups is supported (up to the
    // maximum number of supported patterns), since implicit groups are handled
    // by the search loop itself.
    #[test]
    fn max_slots() {
        // One too many...
        let pat = r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)(q)";
        assert!(DFA::new(pat).is_err());
        // Just right.
        let pat = r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)";
        assert!(DFA::new(pat).is_ok());
    }

    // This test ensures that the one-pass DFA works with all look-around
    // assertions that we expect it to work with.
    //
    // The utility of this test is that each one-pass transition has a small
    // amount of space to store look-around assertions. Currently, there is
    // logic in the one-pass constructor to ensure there aren't more than ten
    // possible assertions. And indeed, there are only ten possible assertions
    // (at time of writing), so this is okay. But conceivably, more assertions
    // could be added. So we check that things at least work with what we
    // expect them to work with.
    #[test]
    fn assertions() {
        // haystack anchors
        assert!(DFA::new(r"^").is_ok());
        assert!(DFA::new(r"$").is_ok());

        // line anchors
        assert!(DFA::new(r"(?m)^").is_ok());
        assert!(DFA::new(r"(?m)$").is_ok());
        assert!(DFA::new(r"(?Rm)^").is_ok());
        assert!(DFA::new(r"(?Rm)$").is_ok());

        // word boundaries
        if cfg!(feature = "unicode-word-boundary") {
            assert!(DFA::new(r"\b").is_ok());
            assert!(DFA::new(r"\B").is_ok());
        }
        assert!(DFA::new(r"(?-u)\b").is_ok());
        assert!(DFA::new(r"(?-u)\B").is_ok());
    }

    #[cfg(not(miri))] // takes too long on miri
    #[test]
    fn is_one_pass() {
        use crate::util::syntax;

        assert!(DFA::new(r"a*b").is_ok());
        if cfg!(feature = "unicode-perl") {
            assert!(DFA::new(r"\w").is_ok());
        }
        assert!(DFA::new(r"(?-u)\w*\s").is_ok());
        assert!(DFA::new(r"(?s:.)*?").is_ok());
        assert!(DFA::builder()
            .syntax(syntax::Config::new().utf8(false))
            .build(r"(?s-u:.)*?")
            .is_ok());
    }

    #[test]
    fn is_not_one_pass() {
        assert!(DFA::new(r"a*a").is_err());
        assert!(DFA::new(r"(?s-u:.)*?").is_err());
        assert!(DFA::new(r"(?s:.)*?a").is_err());
    }

    #[cfg(not(miri))]
    #[test]
    fn is_not_one_pass_bigger() {
        assert!(DFA::new(r"\w*\s").is_err());
    }
}
