#[cfg(all(test, feature = "alloc"))]
mod tests {
    use alloc::{vec, vec::Vec};

    use super::*;

    #[test]
    fn byte_classes() {
        let mut set = ByteClassSet::empty();
        set.set_range(b'a', b'z');

        let classes = set.byte_classes();
        assert_eq!(classes.get(0), 0);
        assert_eq!(classes.get(1), 0);
        assert_eq!(classes.get(2), 0);
        assert_eq!(classes.get(b'a' - 1), 0);
        assert_eq!(classes.get(b'a'), 1);
        assert_eq!(classes.get(b'm'), 1);
        assert_eq!(classes.get(b'z'), 1);
        assert_eq!(classes.get(b'z' + 1), 2);
        assert_eq!(classes.get(254), 2);
        assert_eq!(classes.get(255), 2);

        let mut set = ByteClassSet::empty();
        set.set_range(0, 2);
        set.set_range(4, 6);
        let classes = set.byte_classes();
        assert_eq!(classes.get(0), 0);
        assert_eq!(classes.get(1), 0);
        assert_eq!(classes.get(2), 0);
        assert_eq!(classes.get(3), 1);
        assert_eq!(classes.get(4), 2);
        assert_eq!(classes.get(5), 2);
        assert_eq!(classes.get(6), 2);
        assert_eq!(classes.get(7), 3);
        assert_eq!(classes.get(255), 3);
    }

    #[test]
    fn full_byte_classes() {
        let mut set = ByteClassSet::empty();
        for b in 0u8..=255 {
            set.set_range(b, b);
        }
        assert_eq!(set.byte_classes().alphabet_len(), 257);
    }

    #[test]
    fn elements_typical() {
        let mut set = ByteClassSet::empty();
        set.set_range(b'b', b'd');
        set.set_range(b'g', b'm');
        set.set_range(b'z', b'z');
        let classes = set.byte_classes();
        // class 0: \x00-a
        // class 1: b-d
        // class 2: e-f
        // class 3: g-m
        // class 4: n-y
        // class 5: z-z
        // class 6: \x7B-\xFF
        // class 7: EOI
        assert_eq!(classes.alphabet_len(), 8);

        let elements = classes.elements(Unit::u8(0)).collect::<Vec<_>>();
        assert_eq!(elements.len(), 98);
        assert_eq!(elements[0], Unit::u8(b'\x00'));
        assert_eq!(elements[97], Unit::u8(b'a'));

        let elements = classes.elements(Unit::u8(1)).collect::<Vec<_>>();
        assert_eq!(
            elements,
            vec![Unit::u8(b'b'), Unit::u8(b'c'), Unit::u8(b'd')],
        );

        let elements = classes.elements(Unit::u8(2)).collect::<Vec<_>>();
        assert_eq!(elements, vec![Unit::u8(b'e'), Unit::u8(b'f')],);

        let elements = classes.elements(Unit::u8(3)).collect::<Vec<_>>();
        assert_eq!(
            elements,
            vec![
                Unit::u8(b'g'),
                Unit::u8(b'h'),
                Unit::u8(b'i'),
                Unit::u8(b'j'),
                Unit::u8(b'k'),
                Unit::u8(b'l'),
                Unit::u8(b'm'),
            ],
        );

        let elements = classes.elements(Unit::u8(4)).collect::<Vec<_>>();
        assert_eq!(elements.len(), 12);
        assert_eq!(elements[0], Unit::u8(b'n'));
        assert_eq!(elements[11], Unit::u8(b'y'));

        let elements = classes.elements(Unit::u8(5)).collect::<Vec<_>>();
        assert_eq!(elements, vec![Unit::u8(b'z')]);

        let elements = classes.elements(Unit::u8(6)).collect::<Vec<_>>();
        assert_eq!(elements.len(), 133);
        assert_eq!(elements[0], Unit::u8(b'\x7B'));
        assert_eq!(elements[132], Unit::u8(b'\xFF'));

        let elements = classes.elements(Unit::eoi(7)).collect::<Vec<_>>();
        assert_eq!(elements, vec![Unit::eoi(256)]);
    }

    #[test]
    fn elements_singletons() {
        let classes = ByteClasses::singletons();
        assert_eq!(classes.alphabet_len(), 257);

        let elements = classes.elements(Unit::u8(b'a')).collect::<Vec<_>>();
        assert_eq!(elements, vec![Unit::u8(b'a')]);

        let elements = classes.elements(Unit::eoi(5)).collect::<Vec<_>>();
        assert_eq!(elements, vec![Unit::eoi(256)]);
    }

    #[test]
    fn elements_empty() {
        let classes = ByteClasses::empty();
        assert_eq!(classes.alphabet_len(), 2);

        let elements = classes.elements(Unit::u8(0)).collect::<Vec<_>>();
        assert_eq!(elements.len(), 256);
        assert_eq!(elements[0], Unit::u8(b'\x00'));
        assert_eq!(elements[255], Unit::u8(b'\xFF'));

        let elements = classes.elements(Unit::eoi(1)).collect::<Vec<_>>();
        assert_eq!(elements, vec![Unit::eoi(256)]);
    }

    #[test]
    fn representatives() {
        let mut set = ByteClassSet::empty();
        set.set_range(b'b', b'd');
        set.set_range(b'g', b'm');
        set.set_range(b'z', b'z');
        let classes = set.byte_classes();

        let got: Vec<Unit> = classes.representatives(..).collect();
        let expected = vec![
            Unit::u8(b'\x00'),
            Unit::u8(b'b'),
            Unit::u8(b'e'),
            Unit::u8(b'g'),
            Unit::u8(b'n'),
            Unit::u8(b'z'),
            Unit::u8(b'\x7B'),
            Unit::eoi(7),
        ];
        assert_eq!(expected, got);

        let got: Vec<Unit> = classes.representatives(..0).collect();
        assert!(got.is_empty());
        let got: Vec<Unit> = classes.representatives(1..1).collect();
        assert!(got.is_empty());
        let got: Vec<Unit> = classes.representatives(255..255).collect();
        assert!(got.is_empty());

        // A weird case that is the only guaranteed to way to get an iterator
        // of just the EOI class by excluding all possible byte values.
        let got: Vec<Unit> = classes
            .representatives((
                core::ops::Bound::Excluded(255),
                core::ops::Bound::Unbounded,
            ))
            .collect();
        let expected = vec![Unit::eoi(7)];
        assert_eq!(expected, got);

        let got: Vec<Unit> = classes.representatives(..=255).collect();
        let expected = vec![
            Unit::u8(b'\x00'),
            Unit::u8(b'b'),
            Unit::u8(b'e'),
            Unit::u8(b'g'),
            Unit::u8(b'n'),
            Unit::u8(b'z'),
            Unit::u8(b'\x7B'),
        ];
        assert_eq!(expected, got);

        let got: Vec<Unit> = classes.representatives(b'b'..=b'd').collect();
        let expected = vec![Unit::u8(b'b')];
        assert_eq!(expected, got);

        let got: Vec<Unit> = classes.representatives(b'a'..=b'd').collect();
        let expected = vec![Unit::u8(b'a'), Unit::u8(b'b')];
        assert_eq!(expected, got);

        let got: Vec<Unit> = classes.representatives(b'b'..=b'e').collect();
        let expected = vec![Unit::u8(b'b'), Unit::u8(b'e')];
        assert_eq!(expected, got);

        let got: Vec<Unit> = classes.representatives(b'A'..=b'Z').collect();
        let expected = vec![Unit::u8(b'A')];
        assert_eq!(expected, got);

        let got: Vec<Unit> = classes.representatives(b'A'..=b'z').collect();
        let expected = vec![
            Unit::u8(b'A'),
            Unit::u8(b'b'),
            Unit::u8(b'e'),
            Unit::u8(b'g'),
            Unit::u8(b'n'),
            Unit::u8(b'z'),
        ];
        assert_eq!(expected, got);

        let got: Vec<Unit> = classes.representatives(b'z'..).collect();
        let expected = vec![Unit::u8(b'z'), Unit::u8(b'\x7B'), Unit::eoi(7)];
        assert_eq!(expected, got);

        let got: Vec<Unit> = classes.representatives(b'z'..=0xFF).collect();
        let expected = vec![Unit::u8(b'z'), Unit::u8(b'\x7B')];
        assert_eq!(expected, got);
    }
}
