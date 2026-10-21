#[cfg(test)]
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
        assert_eq!(set.byte_classes().alphabet_len(), 256);
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
        assert_eq!(classes.alphabet_len(), 7);

        let elements = classes.elements(0).collect::<Vec<_>>();
        assert_eq!(elements.len(), 98);
        assert_eq!(elements[0], b'\x00');
        assert_eq!(elements[97], b'a');

        let elements = classes.elements(1).collect::<Vec<_>>();
        assert_eq!(elements, vec![b'b', b'c', b'd'],);

        let elements = classes.elements(2).collect::<Vec<_>>();
        assert_eq!(elements, vec![b'e', b'f'],);

        let elements = classes.elements(3).collect::<Vec<_>>();
        assert_eq!(elements, vec![b'g', b'h', b'i', b'j', b'k', b'l', b'm',],);

        let elements = classes.elements(4).collect::<Vec<_>>();
        assert_eq!(elements.len(), 12);
        assert_eq!(elements[0], b'n');
        assert_eq!(elements[11], b'y');

        let elements = classes.elements(5).collect::<Vec<_>>();
        assert_eq!(elements, vec![b'z']);

        let elements = classes.elements(6).collect::<Vec<_>>();
        assert_eq!(elements.len(), 133);
        assert_eq!(elements[0], b'\x7B');
        assert_eq!(elements[132], b'\xFF');
    }

    #[test]
    fn elements_singletons() {
        let classes = ByteClasses::singletons();
        assert_eq!(classes.alphabet_len(), 256);

        let elements = classes.elements(b'a').collect::<Vec<_>>();
        assert_eq!(elements, vec![b'a']);
    }

    #[test]
    fn elements_empty() {
        let classes = ByteClasses::empty();
        assert_eq!(classes.alphabet_len(), 1);

        let elements = classes.elements(0).collect::<Vec<_>>();
        assert_eq!(elements.len(), 256);
        assert_eq!(elements[0], b'\x00');
        assert_eq!(elements[255], b'\xFF');
    }
}
