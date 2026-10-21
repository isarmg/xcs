#[cfg(test)]
mod tests {
    use super::*;

    fn uclass(ranges: &[(char, char)]) -> ClassUnicode {
        let ranges: Vec<ClassUnicodeRange> = ranges
            .iter()
            .map(|&(s, e)| ClassUnicodeRange::new(s, e))
            .collect();
        ClassUnicode::new(ranges)
    }

    fn bclass(ranges: &[(u8, u8)]) -> ClassBytes {
        let ranges: Vec<ClassBytesRange> =
            ranges.iter().map(|&(s, e)| ClassBytesRange::new(s, e)).collect();
        ClassBytes::new(ranges)
    }

    fn uranges(cls: &ClassUnicode) -> Vec<(char, char)> {
        cls.iter().map(|x| (x.start(), x.end())).collect()
    }

    #[cfg(feature = "unicode-case")]
    fn ucasefold(cls: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls.clone();
        cls_.case_fold_simple();
        cls_
    }

    fn uunion(cls1: &ClassUnicode, cls2: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls1.clone();
        cls_.union(cls2);
        cls_
    }

    fn uintersect(cls1: &ClassUnicode, cls2: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls1.clone();
        cls_.intersect(cls2);
        cls_
    }

    fn udifference(cls1: &ClassUnicode, cls2: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls1.clone();
        cls_.difference(cls2);
        cls_
    }

    fn usymdifference(
        cls1: &ClassUnicode,
        cls2: &ClassUnicode,
    ) -> ClassUnicode {
        let mut cls_ = cls1.clone();
        cls_.symmetric_difference(cls2);
        cls_
    }

    fn unegate(cls: &ClassUnicode) -> ClassUnicode {
        let mut cls_ = cls.clone();
        cls_.negate();
        cls_
    }

    fn branges(cls: &ClassBytes) -> Vec<(u8, u8)> {
        cls.iter().map(|x| (x.start(), x.end())).collect()
    }

    fn bcasefold(cls: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls.clone();
        cls_.case_fold_simple();
        cls_
    }

    fn bunion(cls1: &ClassBytes, cls2: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls1.clone();
        cls_.union(cls2);
        cls_
    }

    fn bintersect(cls1: &ClassBytes, cls2: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls1.clone();
        cls_.intersect(cls2);
        cls_
    }

    fn bdifference(cls1: &ClassBytes, cls2: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls1.clone();
        cls_.difference(cls2);
        cls_
    }

    fn bsymdifference(cls1: &ClassBytes, cls2: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls1.clone();
        cls_.symmetric_difference(cls2);
        cls_
    }

    fn bnegate(cls: &ClassBytes) -> ClassBytes {
        let mut cls_ = cls.clone();
        cls_.negate();
        cls_
    }

    #[test]
    fn class_range_canonical_unicode() {
        let range = ClassUnicodeRange::new('\u{00FF}', '\0');
        assert_eq!('\0', range.start());
        assert_eq!('\u{00FF}', range.end());
    }

    #[test]
    fn class_range_canonical_bytes() {
        let range = ClassBytesRange::new(b'\xFF', b'\0');
        assert_eq!(b'\0', range.start());
        assert_eq!(b'\xFF', range.end());
    }

    #[test]
    fn class_canonicalize_unicode() {
        let cls = uclass(&[('a', 'c'), ('x', 'z')]);
        let expected = vec![('a', 'c'), ('x', 'z')];
        assert_eq!(expected, uranges(&cls));

        let cls = uclass(&[('x', 'z'), ('a', 'c')]);
        let expected = vec![('a', 'c'), ('x', 'z')];
        assert_eq!(expected, uranges(&cls));

        let cls = uclass(&[('x', 'z'), ('w', 'y')]);
        let expected = vec![('w', 'z')];
        assert_eq!(expected, uranges(&cls));

        let cls = uclass(&[
            ('c', 'f'),
            ('a', 'g'),
            ('d', 'j'),
            ('a', 'c'),
            ('m', 'p'),
            ('l', 's'),
        ]);
        let expected = vec![('a', 'j'), ('l', 's')];
        assert_eq!(expected, uranges(&cls));

        let cls = uclass(&[('x', 'z'), ('u', 'w')]);
        let expected = vec![('u', 'z')];
        assert_eq!(expected, uranges(&cls));

        let cls = uclass(&[('\x00', '\u{10FFFF}'), ('\x00', '\u{10FFFF}')]);
        let expected = vec![('\x00', '\u{10FFFF}')];
        assert_eq!(expected, uranges(&cls));

        let cls = uclass(&[('a', 'a'), ('b', 'b')]);
        let expected = vec![('a', 'b')];
        assert_eq!(expected, uranges(&cls));
    }

    #[test]
    fn class_canonicalize_bytes() {
        let cls = bclass(&[(b'a', b'c'), (b'x', b'z')]);
        let expected = vec![(b'a', b'c'), (b'x', b'z')];
        assert_eq!(expected, branges(&cls));

        let cls = bclass(&[(b'x', b'z'), (b'a', b'c')]);
        let expected = vec![(b'a', b'c'), (b'x', b'z')];
        assert_eq!(expected, branges(&cls));

        let cls = bclass(&[(b'x', b'z'), (b'w', b'y')]);
        let expected = vec![(b'w', b'z')];
        assert_eq!(expected, branges(&cls));

        let cls = bclass(&[
            (b'c', b'f'),
            (b'a', b'g'),
            (b'd', b'j'),
            (b'a', b'c'),
            (b'm', b'p'),
            (b'l', b's'),
        ]);
        let expected = vec![(b'a', b'j'), (b'l', b's')];
        assert_eq!(expected, branges(&cls));

        let cls = bclass(&[(b'x', b'z'), (b'u', b'w')]);
        let expected = vec![(b'u', b'z')];
        assert_eq!(expected, branges(&cls));

        let cls = bclass(&[(b'\x00', b'\xFF'), (b'\x00', b'\xFF')]);
        let expected = vec![(b'\x00', b'\xFF')];
        assert_eq!(expected, branges(&cls));

        let cls = bclass(&[(b'a', b'a'), (b'b', b'b')]);
        let expected = vec![(b'a', b'b')];
        assert_eq!(expected, branges(&cls));
    }

    #[test]
    #[cfg(feature = "unicode-case")]
    fn class_case_fold_unicode() {
        let cls = uclass(&[
            ('C', 'F'),
            ('A', 'G'),
            ('D', 'J'),
            ('A', 'C'),
            ('M', 'P'),
            ('L', 'S'),
            ('c', 'f'),
        ]);
        let expected = uclass(&[
            ('A', 'J'),
            ('L', 'S'),
            ('a', 'j'),
            ('l', 's'),
            ('\u{17F}', '\u{17F}'),
        ]);
        assert_eq!(expected, ucasefold(&cls));

        let cls = uclass(&[('A', 'Z')]);
        let expected = uclass(&[
            ('A', 'Z'),
            ('a', 'z'),
            ('\u{17F}', '\u{17F}'),
            ('\u{212A}', '\u{212A}'),
        ]);
        assert_eq!(expected, ucasefold(&cls));

        let cls = uclass(&[('a', 'z')]);
        let expected = uclass(&[
            ('A', 'Z'),
            ('a', 'z'),
            ('\u{17F}', '\u{17F}'),
            ('\u{212A}', '\u{212A}'),
        ]);
        assert_eq!(expected, ucasefold(&cls));

        let cls = uclass(&[('A', 'A'), ('_', '_')]);
        let expected = uclass(&[('A', 'A'), ('_', '_'), ('a', 'a')]);
        assert_eq!(expected, ucasefold(&cls));

        let cls = uclass(&[('A', 'A'), ('=', '=')]);
        let expected = uclass(&[('=', '='), ('A', 'A'), ('a', 'a')]);
        assert_eq!(expected, ucasefold(&cls));

        let cls = uclass(&[('\x00', '\x10')]);
        assert_eq!(cls, ucasefold(&cls));

        let cls = uclass(&[('k', 'k')]);
        let expected =
            uclass(&[('K', 'K'), ('k', 'k'), ('\u{212A}', '\u{212A}')]);
        assert_eq!(expected, ucasefold(&cls));

        let cls = uclass(&[('@', '@')]);
        assert_eq!(cls, ucasefold(&cls));
    }

    #[test]
    #[cfg(not(feature = "unicode-case"))]
    fn class_case_fold_unicode_disabled() {
        let mut cls = uclass(&[
            ('C', 'F'),
            ('A', 'G'),
            ('D', 'J'),
            ('A', 'C'),
            ('M', 'P'),
            ('L', 'S'),
            ('c', 'f'),
        ]);
        assert!(cls.try_case_fold_simple().is_err());
    }

    #[test]
    #[should_panic]
    #[cfg(not(feature = "unicode-case"))]
    fn class_case_fold_unicode_disabled_panics() {
        let mut cls = uclass(&[
            ('C', 'F'),
            ('A', 'G'),
            ('D', 'J'),
            ('A', 'C'),
            ('M', 'P'),
            ('L', 'S'),
            ('c', 'f'),
        ]);
        cls.case_fold_simple();
    }

    #[test]
    fn class_case_fold_bytes() {
        let cls = bclass(&[
            (b'C', b'F'),
            (b'A', b'G'),
            (b'D', b'J'),
            (b'A', b'C'),
            (b'M', b'P'),
            (b'L', b'S'),
            (b'c', b'f'),
        ]);
        let expected =
            bclass(&[(b'A', b'J'), (b'L', b'S'), (b'a', b'j'), (b'l', b's')]);
        assert_eq!(expected, bcasefold(&cls));

        let cls = bclass(&[(b'A', b'Z')]);
        let expected = bclass(&[(b'A', b'Z'), (b'a', b'z')]);
        assert_eq!(expected, bcasefold(&cls));

        let cls = bclass(&[(b'a', b'z')]);
        let expected = bclass(&[(b'A', b'Z'), (b'a', b'z')]);
        assert_eq!(expected, bcasefold(&cls));

        let cls = bclass(&[(b'A', b'A'), (b'_', b'_')]);
        let expected = bclass(&[(b'A', b'A'), (b'_', b'_'), (b'a', b'a')]);
        assert_eq!(expected, bcasefold(&cls));

        let cls = bclass(&[(b'A', b'A'), (b'=', b'=')]);
        let expected = bclass(&[(b'=', b'='), (b'A', b'A'), (b'a', b'a')]);
        assert_eq!(expected, bcasefold(&cls));

        let cls = bclass(&[(b'\x00', b'\x10')]);
        assert_eq!(cls, bcasefold(&cls));

        let cls = bclass(&[(b'k', b'k')]);
        let expected = bclass(&[(b'K', b'K'), (b'k', b'k')]);
        assert_eq!(expected, bcasefold(&cls));

        let cls = bclass(&[(b'@', b'@')]);
        assert_eq!(cls, bcasefold(&cls));
    }

    #[test]
    fn class_negate_unicode() {
        let cls = uclass(&[('a', 'a')]);
        let expected = uclass(&[('\x00', '\x60'), ('\x62', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(&cls));

        let cls = uclass(&[('a', 'a'), ('b', 'b')]);
        let expected = uclass(&[('\x00', '\x60'), ('\x63', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(&cls));

        let cls = uclass(&[('a', 'c'), ('x', 'z')]);
        let expected = uclass(&[
            ('\x00', '\x60'),
            ('\x64', '\x77'),
            ('\x7B', '\u{10FFFF}'),
        ]);
        assert_eq!(expected, unegate(&cls));

        let cls = uclass(&[('\x00', 'a')]);
        let expected = uclass(&[('\x62', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(&cls));

        let cls = uclass(&[('a', '\u{10FFFF}')]);
        let expected = uclass(&[('\x00', '\x60')]);
        assert_eq!(expected, unegate(&cls));

        let cls = uclass(&[('\x00', '\u{10FFFF}')]);
        let expected = uclass(&[]);
        assert_eq!(expected, unegate(&cls));

        let cls = uclass(&[]);
        let expected = uclass(&[('\x00', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(&cls));

        let cls =
            uclass(&[('\x00', '\u{10FFFD}'), ('\u{10FFFF}', '\u{10FFFF}')]);
        let expected = uclass(&[('\u{10FFFE}', '\u{10FFFE}')]);
        assert_eq!(expected, unegate(&cls));

        let cls = uclass(&[('\x00', '\u{D7FF}')]);
        let expected = uclass(&[('\u{E000}', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(&cls));

        let cls = uclass(&[('\x00', '\u{D7FE}')]);
        let expected = uclass(&[('\u{D7FF}', '\u{10FFFF}')]);
        assert_eq!(expected, unegate(&cls));

        let cls = uclass(&[('\u{E000}', '\u{10FFFF}')]);
        let expected = uclass(&[('\x00', '\u{D7FF}')]);
        assert_eq!(expected, unegate(&cls));

        let cls = uclass(&[('\u{E001}', '\u{10FFFF}')]);
        let expected = uclass(&[('\x00', '\u{E000}')]);
        assert_eq!(expected, unegate(&cls));
    }

    #[test]
    fn class_negate_bytes() {
        let cls = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[(b'\x00', b'\x60'), (b'\x62', b'\xFF')]);
        assert_eq!(expected, bnegate(&cls));

        let cls = bclass(&[(b'a', b'a'), (b'b', b'b')]);
        let expected = bclass(&[(b'\x00', b'\x60'), (b'\x63', b'\xFF')]);
        assert_eq!(expected, bnegate(&cls));

        let cls = bclass(&[(b'a', b'c'), (b'x', b'z')]);
        let expected = bclass(&[
            (b'\x00', b'\x60'),
            (b'\x64', b'\x77'),
            (b'\x7B', b'\xFF'),
        ]);
        assert_eq!(expected, bnegate(&cls));

        let cls = bclass(&[(b'\x00', b'a')]);
        let expected = bclass(&[(b'\x62', b'\xFF')]);
        assert_eq!(expected, bnegate(&cls));

        let cls = bclass(&[(b'a', b'\xFF')]);
        let expected = bclass(&[(b'\x00', b'\x60')]);
        assert_eq!(expected, bnegate(&cls));

        let cls = bclass(&[(b'\x00', b'\xFF')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bnegate(&cls));

        let cls = bclass(&[]);
        let expected = bclass(&[(b'\x00', b'\xFF')]);
        assert_eq!(expected, bnegate(&cls));

        let cls = bclass(&[(b'\x00', b'\xFD'), (b'\xFF', b'\xFF')]);
        let expected = bclass(&[(b'\xFE', b'\xFE')]);
        assert_eq!(expected, bnegate(&cls));
    }

    #[test]
    fn class_union_unicode() {
        let cls1 = uclass(&[('a', 'g'), ('m', 't'), ('A', 'C')]);
        let cls2 = uclass(&[('a', 'z')]);
        let expected = uclass(&[('a', 'z'), ('A', 'C')]);
        assert_eq!(expected, uunion(&cls1, &cls2));
    }

    #[test]
    fn class_union_bytes() {
        let cls1 = bclass(&[(b'a', b'g'), (b'm', b't'), (b'A', b'C')]);
        let cls2 = bclass(&[(b'a', b'z')]);
        let expected = bclass(&[(b'a', b'z'), (b'A', b'C')]);
        assert_eq!(expected, bunion(&cls1, &cls2));
    }

    #[test]
    fn class_intersect_unicode() {
        let cls1 = uclass(&[]);
        let cls2 = uclass(&[('a', 'a')]);
        let expected = uclass(&[]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'a')]);
        let cls2 = uclass(&[('a', 'a')]);
        let expected = uclass(&[('a', 'a')]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'a')]);
        let cls2 = uclass(&[('b', 'b')]);
        let expected = uclass(&[]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'a')]);
        let cls2 = uclass(&[('a', 'c')]);
        let expected = uclass(&[('a', 'a')]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'b')]);
        let cls2 = uclass(&[('a', 'c')]);
        let expected = uclass(&[('a', 'b')]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'b')]);
        let cls2 = uclass(&[('b', 'c')]);
        let expected = uclass(&[('b', 'b')]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'b')]);
        let cls2 = uclass(&[('c', 'd')]);
        let expected = uclass(&[]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('b', 'c')]);
        let cls2 = uclass(&[('a', 'd')]);
        let expected = uclass(&[('b', 'c')]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        let cls2 = uclass(&[('a', 'h')]);
        let expected = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        let cls2 = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        let expected = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'b'), ('g', 'h')]);
        let cls2 = uclass(&[('d', 'e'), ('k', 'l')]);
        let expected = uclass(&[]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'b'), ('d', 'e'), ('g', 'h')]);
        let cls2 = uclass(&[('h', 'h')]);
        let expected = uclass(&[('h', 'h')]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'b'), ('e', 'f'), ('i', 'j')]);
        let cls2 = uclass(&[('c', 'd'), ('g', 'h'), ('k', 'l')]);
        let expected = uclass(&[]);
        assert_eq!(expected, uintersect(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'b'), ('c', 'd'), ('e', 'f')]);
        let cls2 = uclass(&[('b', 'c'), ('d', 'e'), ('f', 'g')]);
        let expected = uclass(&[('b', 'f')]);
        assert_eq!(expected, uintersect(&cls1, &cls2));
    }

    #[test]
    fn class_intersect_bytes() {
        let cls1 = bclass(&[]);
        let cls2 = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'a')]);
        let cls2 = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[(b'a', b'a')]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'a')]);
        let cls2 = bclass(&[(b'b', b'b')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'a')]);
        let cls2 = bclass(&[(b'a', b'c')]);
        let expected = bclass(&[(b'a', b'a')]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'b')]);
        let cls2 = bclass(&[(b'a', b'c')]);
        let expected = bclass(&[(b'a', b'b')]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'b')]);
        let cls2 = bclass(&[(b'b', b'c')]);
        let expected = bclass(&[(b'b', b'b')]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'b')]);
        let cls2 = bclass(&[(b'c', b'd')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'b', b'c')]);
        let cls2 = bclass(&[(b'a', b'd')]);
        let expected = bclass(&[(b'b', b'c')]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        let cls2 = bclass(&[(b'a', b'h')]);
        let expected = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        let cls2 = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        let expected = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'b'), (b'g', b'h')]);
        let cls2 = bclass(&[(b'd', b'e'), (b'k', b'l')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'b'), (b'd', b'e'), (b'g', b'h')]);
        let cls2 = bclass(&[(b'h', b'h')]);
        let expected = bclass(&[(b'h', b'h')]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'b'), (b'e', b'f'), (b'i', b'j')]);
        let cls2 = bclass(&[(b'c', b'd'), (b'g', b'h'), (b'k', b'l')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bintersect(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'b'), (b'c', b'd'), (b'e', b'f')]);
        let cls2 = bclass(&[(b'b', b'c'), (b'd', b'e'), (b'f', b'g')]);
        let expected = bclass(&[(b'b', b'f')]);
        assert_eq!(expected, bintersect(&cls1, &cls2));
    }

    #[test]
    fn class_difference_unicode() {
        let cls1 = uclass(&[('a', 'a')]);
        let cls2 = uclass(&[('a', 'a')]);
        let expected = uclass(&[]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'a')]);
        let cls2 = uclass(&[]);
        let expected = uclass(&[('a', 'a')]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[]);
        let cls2 = uclass(&[('a', 'a')]);
        let expected = uclass(&[]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'z')]);
        let cls2 = uclass(&[('a', 'a')]);
        let expected = uclass(&[('b', 'z')]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'z')]);
        let cls2 = uclass(&[('z', 'z')]);
        let expected = uclass(&[('a', 'y')]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'z')]);
        let cls2 = uclass(&[('m', 'm')]);
        let expected = uclass(&[('a', 'l'), ('n', 'z')]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'c'), ('g', 'i'), ('r', 't')]);
        let cls2 = uclass(&[('a', 'z')]);
        let expected = uclass(&[]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'c'), ('g', 'i'), ('r', 't')]);
        let cls2 = uclass(&[('d', 'v')]);
        let expected = uclass(&[('a', 'c')]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'c'), ('g', 'i'), ('r', 't')]);
        let cls2 = uclass(&[('b', 'g'), ('s', 'u')]);
        let expected = uclass(&[('a', 'a'), ('h', 'i'), ('r', 'r')]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'c'), ('g', 'i'), ('r', 't')]);
        let cls2 = uclass(&[('b', 'd'), ('e', 'g'), ('s', 'u')]);
        let expected = uclass(&[('a', 'a'), ('h', 'i'), ('r', 'r')]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[('x', 'z')]);
        let cls2 = uclass(&[('a', 'c'), ('e', 'g'), ('s', 'u')]);
        let expected = uclass(&[('x', 'z')]);
        assert_eq!(expected, udifference(&cls1, &cls2));

        let cls1 = uclass(&[('a', 'z')]);
        let cls2 = uclass(&[('a', 'c'), ('e', 'g'), ('s', 'u')]);
        let expected = uclass(&[('d', 'd'), ('h', 'r'), ('v', 'z')]);
        assert_eq!(expected, udifference(&cls1, &cls2));
    }

    #[test]
    fn class_difference_bytes() {
        let cls1 = bclass(&[(b'a', b'a')]);
        let cls2 = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'a')]);
        let cls2 = bclass(&[]);
        let expected = bclass(&[(b'a', b'a')]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[]);
        let cls2 = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'z')]);
        let cls2 = bclass(&[(b'a', b'a')]);
        let expected = bclass(&[(b'b', b'z')]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'z')]);
        let cls2 = bclass(&[(b'z', b'z')]);
        let expected = bclass(&[(b'a', b'y')]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'z')]);
        let cls2 = bclass(&[(b'm', b'm')]);
        let expected = bclass(&[(b'a', b'l'), (b'n', b'z')]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'c'), (b'g', b'i'), (b'r', b't')]);
        let cls2 = bclass(&[(b'a', b'z')]);
        let expected = bclass(&[]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'c'), (b'g', b'i'), (b'r', b't')]);
        let cls2 = bclass(&[(b'd', b'v')]);
        let expected = bclass(&[(b'a', b'c')]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'c'), (b'g', b'i'), (b'r', b't')]);
        let cls2 = bclass(&[(b'b', b'g'), (b's', b'u')]);
        let expected = bclass(&[(b'a', b'a'), (b'h', b'i'), (b'r', b'r')]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'c'), (b'g', b'i'), (b'r', b't')]);
        let cls2 = bclass(&[(b'b', b'd'), (b'e', b'g'), (b's', b'u')]);
        let expected = bclass(&[(b'a', b'a'), (b'h', b'i'), (b'r', b'r')]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[(b'x', b'z')]);
        let cls2 = bclass(&[(b'a', b'c'), (b'e', b'g'), (b's', b'u')]);
        let expected = bclass(&[(b'x', b'z')]);
        assert_eq!(expected, bdifference(&cls1, &cls2));

        let cls1 = bclass(&[(b'a', b'z')]);
        let cls2 = bclass(&[(b'a', b'c'), (b'e', b'g'), (b's', b'u')]);
        let expected = bclass(&[(b'd', b'd'), (b'h', b'r'), (b'v', b'z')]);
        assert_eq!(expected, bdifference(&cls1, &cls2));
    }

    #[test]
    fn class_symmetric_difference_unicode() {
        let cls1 = uclass(&[('a', 'm')]);
        let cls2 = uclass(&[('g', 't')]);
        let expected = uclass(&[('a', 'f'), ('n', 't')]);
        assert_eq!(expected, usymdifference(&cls1, &cls2));
    }

    #[test]
    fn class_symmetric_difference_bytes() {
        let cls1 = bclass(&[(b'a', b'm')]);
        let cls2 = bclass(&[(b'g', b't')]);
        let expected = bclass(&[(b'a', b'f'), (b'n', b't')]);
        assert_eq!(expected, bsymdifference(&cls1, &cls2));
    }

    // We use a thread with an explicit stack size to test that our destructor
    // for Hir can handle arbitrarily sized expressions in constant stack
    // space. In case we run on a platform without threads (WASM?), we limit
    // this test to Windows/Unix.
    #[test]
    #[cfg(any(unix, windows))]
    fn no_stack_overflow_on_drop() {
        use std::thread;

        let run = || {
            let mut expr = Hir::empty();
            for _ in 0..100 {
                expr = Hir::capture(Capture {
                    index: 1,
                    name: None,
                    sub: Box::new(expr),
                });
                expr = Hir::repetition(Repetition {
                    min: 0,
                    max: Some(1),
                    greedy: true,
                    sub: Box::new(expr),
                });

                expr = Hir {
                    kind: HirKind::Concat(vec![expr]),
                    props: Properties::empty(),
                };
                expr = Hir {
                    kind: HirKind::Alternation(vec![expr]),
                    props: Properties::empty(),
                };
            }
            assert!(!matches!(*expr.kind(), HirKind::Empty));
        };

        // We run our test on a thread with a small stack size so we can
        // force the issue more easily.
        //
        // NOTE(2023-03-21): See the corresponding test in 'crate::syntax::ast::tests'
        // for context on the specific stack size chosen here.
        thread::Builder::new()
            .stack_size(16 << 10)
            .spawn(run)
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn look_set_iter() {
        let set = LookSet::empty();
        assert_eq!(0, set.iter().count());

        let set = LookSet::full();
        assert_eq!(18, set.iter().count());

        let set =
            LookSet::empty().insert(Look::StartLF).insert(Look::WordUnicode);
        assert_eq!(2, set.iter().count());

        let set = LookSet::empty().insert(Look::StartLF);
        assert_eq!(1, set.iter().count());

        let set = LookSet::empty().insert(Look::WordAsciiNegate);
        assert_eq!(1, set.iter().count());
    }

    #[test]
    fn look_set_debug() {
        let res = format!("{:?}", LookSet::empty());
        assert_eq!("∅", res);
        let res = format!("{:?}", LookSet::full());
        assert_eq!("Az^$rRbB𝛃𝚩<>〈〉◁▷◀▶", res);
    }
}
