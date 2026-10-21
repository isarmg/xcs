#[cfg(test)]
mod tests {
    use alloc::vec;

    use crate::{
        nfa::thompson::{SparseTransitions, State},
        util::primitives::SmallIndex,
    };

    use super::*;

    fn build(pattern: &str) -> NFA {
        NFA::compiler()
            .configure(
                NFA::config()
                    .which_captures(WhichCaptures::None)
                    .unanchored_prefix(false),
            )
            .build(pattern)
            .unwrap()
    }

    fn pid(id: usize) -> PatternID {
        PatternID::new(id).unwrap()
    }

    fn sid(id: usize) -> StateID {
        StateID::new(id).unwrap()
    }

    fn s_byte(byte: u8, next: usize) -> State {
        let next = sid(next);
        let trans = Transition { start: byte, end: byte, next };
        State::ByteRange { trans }
    }

    fn s_range(start: u8, end: u8, next: usize) -> State {
        let next = sid(next);
        let trans = Transition { start, end, next };
        State::ByteRange { trans }
    }

    fn s_sparse(transitions: &[(u8, u8, usize)]) -> State {
        let transitions = transitions
            .iter()
            .map(|&(start, end, next)| Transition {
                start,
                end,
                next: sid(next),
            })
            .collect();
        State::Sparse(SparseTransitions { transitions })
    }

    fn s_look(look: Look, next: usize) -> State {
        let next = sid(next);
        State::Look { look, next }
    }

    fn s_bin_union(alt1: usize, alt2: usize) -> State {
        State::BinaryUnion { alt1: sid(alt1), alt2: sid(alt2) }
    }

    fn s_union(alts: &[usize]) -> State {
        State::Union {
            alternates: alts
                .iter()
                .map(|&id| sid(id))
                .collect::<Vec<StateID>>()
                .into_boxed_slice(),
        }
    }

    fn s_cap(next: usize, pattern: usize, index: usize, slot: usize) -> State {
        State::Capture {
            next: sid(next),
            pattern_id: pid(pattern),
            group_index: SmallIndex::new(index).unwrap(),
            slot: SmallIndex::new(slot).unwrap(),
        }
    }

    fn s_fail() -> State {
        State::Fail
    }

    fn s_match(id: usize) -> State {
        State::Match { pattern_id: pid(id) }
    }

    // Test that building an unanchored NFA has an appropriate `(?s:.)*?`
    // prefix.
    #[test]
    fn compile_unanchored_prefix() {
        let nfa = NFA::compiler()
            .configure(NFA::config().which_captures(WhichCaptures::None))
            .build(r"a")
            .unwrap();
        assert_eq!(
            nfa.states(),
            &[
                s_bin_union(2, 1),
                s_range(0, 255, 0),
                s_byte(b'a', 3),
                s_match(0),
            ]
        );
    }

    #[test]
    fn compile_no_unanchored_prefix_with_start_anchor() {
        let nfa = NFA::compiler()
            .configure(NFA::config().which_captures(WhichCaptures::None))
            .build(r"^a")
            .unwrap();
        assert_eq!(
            nfa.states(),
            &[s_look(Look::Start, 1), s_byte(b'a', 2), s_match(0)]
        );
    }

    #[test]
    fn compile_yes_unanchored_prefix_with_end_anchor() {
        let nfa = NFA::compiler()
            .configure(NFA::config().which_captures(WhichCaptures::None))
            .build(r"a$")
            .unwrap();
        assert_eq!(
            nfa.states(),
            &[
                s_bin_union(2, 1),
                s_range(0, 255, 0),
                s_byte(b'a', 3),
                s_look(Look::End, 4),
                s_match(0),
            ]
        );
    }

    #[test]
    fn compile_yes_reverse_unanchored_prefix_with_start_anchor() {
        let nfa = NFA::compiler()
            .configure(
                NFA::config()
                    .reverse(true)
                    .which_captures(WhichCaptures::None),
            )
            .build(r"^a")
            .unwrap();
        assert_eq!(
            nfa.states(),
            &[
                s_bin_union(2, 1),
                s_range(0, 255, 0),
                s_byte(b'a', 3),
                // Anchors get flipped in a reverse automaton.
                s_look(Look::End, 4),
                s_match(0),
            ],
        );
    }

    #[test]
    fn compile_no_reverse_unanchored_prefix_with_end_anchor() {
        let nfa = NFA::compiler()
            .configure(
                NFA::config()
                    .reverse(true)
                    .which_captures(WhichCaptures::None),
            )
            .build(r"a$")
            .unwrap();
        assert_eq!(
            nfa.states(),
            &[
                // Anchors get flipped in a reverse automaton.
                s_look(Look::Start, 1),
                s_byte(b'a', 2),
                s_match(0),
            ],
        );
    }

    #[test]
    fn compile_empty() {
        assert_eq!(build("").states(), &[s_match(0),]);
    }

    #[test]
    fn compile_literal() {
        assert_eq!(build("a").states(), &[s_byte(b'a', 1), s_match(0),]);
        assert_eq!(
            build("ab").states(),
            &[s_byte(b'a', 1), s_byte(b'b', 2), s_match(0),]
        );
        assert_eq!(
            build("☃").states(),
            &[s_byte(0xE2, 1), s_byte(0x98, 2), s_byte(0x83, 3), s_match(0)]
        );

        // Check that non-UTF-8 literals work.
        let nfa = NFA::compiler()
            .configure(
                NFA::config()
                    .which_captures(WhichCaptures::None)
                    .unanchored_prefix(false),
            )
            .syntax(crate::util::syntax::Config::new().utf8(false))
            .build(r"(?-u)\xFF")
            .unwrap();
        assert_eq!(nfa.states(), &[s_byte(b'\xFF', 1), s_match(0),]);
    }

    #[test]
    fn compile_class_ascii() {
        assert_eq!(
            build(r"[a-z]").states(),
            &[s_range(b'a', b'z', 1), s_match(0),]
        );
        assert_eq!(
            build(r"[x-za-c]").states(),
            &[s_sparse(&[(b'a', b'c', 1), (b'x', b'z', 1)]), s_match(0)]
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn compile_class_unicode() {
        assert_eq!(
            build(r"[\u03B1-\u03B4]").states(),
            &[s_range(0xB1, 0xB4, 2), s_byte(0xCE, 0), s_match(0)]
        );
        assert_eq!(
            build(r"[\u03B1-\u03B4\u{1F919}-\u{1F91E}]").states(),
            &[
                s_range(0xB1, 0xB4, 5),
                s_range(0x99, 0x9E, 5),
                s_byte(0xA4, 1),
                s_byte(0x9F, 2),
                s_sparse(&[(0xCE, 0xCE, 0), (0xF0, 0xF0, 3)]),
                s_match(0),
            ]
        );
        assert_eq!(
            build(r"[a-z☃]").states(),
            &[
                s_byte(0x83, 3),
                s_byte(0x98, 0),
                s_sparse(&[(b'a', b'z', 3), (0xE2, 0xE2, 1)]),
                s_match(0),
            ]
        );
    }

    #[test]
    fn compile_repetition() {
        assert_eq!(
            build(r"a?").states(),
            &[s_bin_union(1, 2), s_byte(b'a', 2), s_match(0),]
        );
        assert_eq!(
            build(r"a??").states(),
            &[s_bin_union(2, 1), s_byte(b'a', 2), s_match(0),]
        );
    }

    #[test]
    fn compile_group() {
        assert_eq!(
            build(r"ab+").states(),
            &[s_byte(b'a', 1), s_byte(b'b', 2), s_bin_union(1, 3), s_match(0)]
        );
        assert_eq!(
            build(r"(ab)").states(),
            &[s_byte(b'a', 1), s_byte(b'b', 2), s_match(0)]
        );
        assert_eq!(
            build(r"(ab)+").states(),
            &[s_byte(b'a', 1), s_byte(b'b', 2), s_bin_union(0, 3), s_match(0)]
        );
    }

    #[test]
    fn compile_alternation() {
        assert_eq!(
            build(r"a|b").states(),
            &[s_range(b'a', b'b', 1), s_match(0)]
        );
        assert_eq!(
            build(r"ab|cd").states(),
            &[
                s_byte(b'b', 3),
                s_byte(b'd', 3),
                s_sparse(&[(b'a', b'a', 0), (b'c', b'c', 1)]),
                s_match(0)
            ],
        );
        assert_eq!(
            build(r"|b").states(),
            &[s_byte(b'b', 2), s_bin_union(2, 0), s_match(0)]
        );
        assert_eq!(
            build(r"a|").states(),
            &[s_byte(b'a', 2), s_bin_union(0, 2), s_match(0)]
        );
    }

    // This tests the use of a non-binary union, i.e., a state with more than
    // 2 unconditional epsilon transitions. The only place they tend to appear
    // is in reverse NFAs when shrinking is disabled. Otherwise, 'binary-union'
    // and 'sparse' tend to cover all other cases of alternation.
    #[test]
    fn compile_non_binary_union() {
        let nfa = NFA::compiler()
            .configure(
                NFA::config()
                    .which_captures(WhichCaptures::None)
                    .reverse(true)
                    .shrink(false)
                    .unanchored_prefix(false),
            )
            .build(r"[\u1000\u2000\u3000]")
            .unwrap();
        assert_eq!(
            nfa.states(),
            &[
                s_union(&[3, 6, 9]),
                s_byte(0xE1, 10),
                s_byte(0x80, 1),
                s_byte(0x80, 2),
                s_byte(0xE2, 10),
                s_byte(0x80, 4),
                s_byte(0x80, 5),
                s_byte(0xE3, 10),
                s_byte(0x80, 7),
                s_byte(0x80, 8),
                s_match(0),
            ]
        );
    }

    #[test]
    fn compile_many_start_pattern() {
        let nfa = NFA::compiler()
            .configure(
                NFA::config()
                    .which_captures(WhichCaptures::None)
                    .unanchored_prefix(false),
            )
            .build_many(&["a", "b"])
            .unwrap();
        assert_eq!(
            nfa.states(),
            &[
                s_byte(b'a', 1),
                s_match(0),
                s_byte(b'b', 3),
                s_match(1),
                s_bin_union(0, 2),
            ]
        );
        assert_eq!(nfa.start_anchored().as_usize(), 4);
        assert_eq!(nfa.start_unanchored().as_usize(), 4);
        // Test that the start states for each individual pattern are correct.
        assert_eq!(nfa.start_pattern(pid(0)).unwrap(), sid(0));
        assert_eq!(nfa.start_pattern(pid(1)).unwrap(), sid(2));
    }

    // This tests that our compiler can handle an empty character class. At the
    // time of writing, the regex parser forbids it, so the only way to test it
    // is to provide a hand written HIR.
    #[test]
    fn empty_class_bytes() {
        use regex_syntax::hir::{Class, ClassBytes, Hir};

        let hir = Hir::class(Class::Bytes(ClassBytes::new(vec![])));
        let config = NFA::config()
            .which_captures(WhichCaptures::None)
            .unanchored_prefix(false);
        let nfa =
            NFA::compiler().configure(config).build_from_hir(&hir).unwrap();
        assert_eq!(nfa.states(), &[s_fail(), s_match(0)]);
    }

    // Like empty_class_bytes, but for a Unicode class.
    #[test]
    fn empty_class_unicode() {
        use regex_syntax::hir::{Class, ClassUnicode, Hir};

        let hir = Hir::class(Class::Unicode(ClassUnicode::new(vec![])));
        let config = NFA::config()
            .which_captures(WhichCaptures::None)
            .unanchored_prefix(false);
        let nfa =
            NFA::compiler().configure(config).build_from_hir(&hir).unwrap();
        assert_eq!(nfa.states(), &[s_fail(), s_match(0)]);
    }

    #[test]
    fn compile_captures_all() {
        let nfa = NFA::compiler()
            .configure(
                NFA::config()
                    .unanchored_prefix(false)
                    .which_captures(WhichCaptures::All),
            )
            .build("a(b)c")
            .unwrap();
        assert_eq!(
            nfa.states(),
            &[
                s_cap(1, 0, 0, 0),
                s_byte(b'a', 2),
                s_cap(3, 0, 1, 2),
                s_byte(b'b', 4),
                s_cap(5, 0, 1, 3),
                s_byte(b'c', 6),
                s_cap(7, 0, 0, 1),
                s_match(0)
            ]
        );
        let ginfo = nfa.group_info();
        assert_eq!(2, ginfo.all_group_len());
    }

    #[test]
    fn compile_captures_implicit() {
        let nfa = NFA::compiler()
            .configure(
                NFA::config()
                    .unanchored_prefix(false)
                    .which_captures(WhichCaptures::Implicit),
            )
            .build("a(b)c")
            .unwrap();
        assert_eq!(
            nfa.states(),
            &[
                s_cap(1, 0, 0, 0),
                s_byte(b'a', 2),
                s_byte(b'b', 3),
                s_byte(b'c', 4),
                s_cap(5, 0, 0, 1),
                s_match(0)
            ]
        );
        let ginfo = nfa.group_info();
        assert_eq!(1, ginfo.all_group_len());
    }

    #[test]
    fn compile_captures_none() {
        let nfa = NFA::compiler()
            .configure(
                NFA::config()
                    .unanchored_prefix(false)
                    .which_captures(WhichCaptures::None),
            )
            .build("a(b)c")
            .unwrap();
        assert_eq!(
            nfa.states(),
            &[s_byte(b'a', 1), s_byte(b'b', 2), s_byte(b'c', 3), s_match(0)]
        );
        let ginfo = nfa.group_info();
        assert_eq!(0, ginfo.all_group_len());
    }
}
