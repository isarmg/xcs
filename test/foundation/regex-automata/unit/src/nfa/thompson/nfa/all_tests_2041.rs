#[cfg(all(test, feature = "nfa-pikevm"))]
mod tests {
    use super::*;
    use crate::{nfa::thompson::pikevm::PikeVM, Input};

    // This asserts that an NFA state doesn't have its size changed. It is
    // *really* easy to accidentally increase the size, and thus potentially
    // dramatically increase the memory usage of every NFA.
    //
    // This assert doesn't mean we absolutely cannot increase the size of an
    // NFA state. We can. It's just here to make sure we do it knowingly and
    // intentionally.
    #[test]
    fn state_has_small_size() {
        #[cfg(target_pointer_width = "64")]
        assert_eq!(24, core::mem::size_of::<State>());
        #[cfg(target_pointer_width = "32")]
        assert_eq!(20, core::mem::size_of::<State>());
    }

    #[test]
    fn always_match() {
        let re = PikeVM::new_from_nfa(NFA::always_match()).unwrap();
        let mut cache = re.create_cache();
        let mut caps = re.create_captures();
        let mut find = |haystack, start, end| {
            let input = Input::new(haystack).range(start..end);
            re.search(&mut cache, &input, &mut caps);
            caps.get_match().map(|m| m.end())
        };

        assert_eq!(Some(0), find("", 0, 0));
        assert_eq!(Some(0), find("a", 0, 1));
        assert_eq!(Some(1), find("a", 1, 1));
        assert_eq!(Some(0), find("ab", 0, 2));
        assert_eq!(Some(1), find("ab", 1, 2));
        assert_eq!(Some(2), find("ab", 2, 2));
    }

    #[test]
    fn never_match() {
        let re = PikeVM::new_from_nfa(NFA::never_match()).unwrap();
        let mut cache = re.create_cache();
        let mut caps = re.create_captures();
        let mut find = |haystack, start, end| {
            let input = Input::new(haystack).range(start..end);
            re.search(&mut cache, &input, &mut caps);
            caps.get_match().map(|m| m.end())
        };

        assert_eq!(None, find("", 0, 0));
        assert_eq!(None, find("a", 0, 1));
        assert_eq!(None, find("a", 1, 1));
        assert_eq!(None, find("ab", 0, 2));
        assert_eq!(None, find("ab", 1, 2));
        assert_eq!(None, find("ab", 2, 2));
    }
}
