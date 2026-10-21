#[cfg(test)]
mod tests {
    use core::char;

    use alloc::{vec, vec::Vec};

    use crate::syntax::utf8::{Utf8Range, Utf8Sequences};

    fn rutf8(s: u8, e: u8) -> Utf8Range {
        Utf8Range::new(s, e)
    }

    fn never_accepts_surrogate_codepoints(start: char, end: char) {
        for cp in 0xD800..0xE000 {
            let buf = encode_surrogate(cp);
            for r in Utf8Sequences::new(start, end) {
                if r.matches(&buf) {
                    panic!(
                        "Sequence ({:X}, {:X}) contains range {:?}, \
                         which matches surrogate code point {:X} \
                         with encoded bytes {:?}",
                        u32::from(start),
                        u32::from(end),
                        r,
                        cp,
                        buf,
                    );
                }
            }
        }
    }

    #[test]
    fn codepoints_no_surrogates() {
        never_accepts_surrogate_codepoints('\u{0}', '\u{FFFF}');
        never_accepts_surrogate_codepoints('\u{0}', '\u{10FFFF}');
        never_accepts_surrogate_codepoints('\u{0}', '\u{10FFFE}');
        never_accepts_surrogate_codepoints('\u{80}', '\u{10FFFF}');
        never_accepts_surrogate_codepoints('\u{D7FF}', '\u{E000}');
    }

    #[test]
    fn single_codepoint_one_sequence() {
        // Tests that every range of scalar values that contains a single
        // scalar value is recognized by one sequence of byte ranges.
        for i in 0x0..=0x0010_FFFF {
            let c = match char::from_u32(i) {
                None => continue,
                Some(c) => c,
            };
            let seqs: Vec<_> = Utf8Sequences::new(c, c).collect();
            assert_eq!(seqs.len(), 1);
        }
    }

    #[test]
    fn bmp() {
        use crate::syntax::utf8::Utf8Sequence::*;

        let seqs = Utf8Sequences::new('\u{0}', '\u{FFFF}').collect::<Vec<_>>();
        assert_eq!(
            seqs,
            vec![
                One(rutf8(0x0, 0x7F)),
                Two([rutf8(0xC2, 0xDF), rutf8(0x80, 0xBF)]),
                Three([
                    rutf8(0xE0, 0xE0),
                    rutf8(0xA0, 0xBF),
                    rutf8(0x80, 0xBF)
                ]),
                Three([
                    rutf8(0xE1, 0xEC),
                    rutf8(0x80, 0xBF),
                    rutf8(0x80, 0xBF)
                ]),
                Three([
                    rutf8(0xED, 0xED),
                    rutf8(0x80, 0x9F),
                    rutf8(0x80, 0xBF)
                ]),
                Three([
                    rutf8(0xEE, 0xEF),
                    rutf8(0x80, 0xBF),
                    rutf8(0x80, 0xBF)
                ]),
            ]
        );
    }

    #[test]
    fn reverse() {
        use crate::syntax::utf8::Utf8Sequence::*;

        let mut s = One(rutf8(0xA, 0xB));
        s.reverse();
        assert_eq!(s.as_slice(), &[rutf8(0xA, 0xB)]);

        let mut s = Two([rutf8(0xA, 0xB), rutf8(0xB, 0xC)]);
        s.reverse();
        assert_eq!(s.as_slice(), &[rutf8(0xB, 0xC), rutf8(0xA, 0xB)]);

        let mut s = Three([rutf8(0xA, 0xB), rutf8(0xB, 0xC), rutf8(0xC, 0xD)]);
        s.reverse();
        assert_eq!(
            s.as_slice(),
            &[rutf8(0xC, 0xD), rutf8(0xB, 0xC), rutf8(0xA, 0xB)]
        );

        let mut s = Four([
            rutf8(0xA, 0xB),
            rutf8(0xB, 0xC),
            rutf8(0xC, 0xD),
            rutf8(0xD, 0xE),
        ]);
        s.reverse();
        assert_eq!(
            s.as_slice(),
            &[
                rutf8(0xD, 0xE),
                rutf8(0xC, 0xD),
                rutf8(0xB, 0xC),
                rutf8(0xA, 0xB)
            ]
        );
    }

    fn encode_surrogate(cp: u32) -> [u8; 3] {
        const TAG_CONT: u8 = 0b1000_0000;
        const TAG_THREE_B: u8 = 0b1110_0000;

        assert!(0xD800 <= cp && cp < 0xE000);
        let mut dst = [0; 3];
        dst[0] = u8::try_from(cp >> 12 & 0x0F).unwrap() | TAG_THREE_B;
        dst[1] = u8::try_from(cp >> 6 & 0x3F).unwrap() | TAG_CONT;
        dst[2] = u8::try_from(cp & 0x3F).unwrap() | TAG_CONT;
        dst
    }
}
