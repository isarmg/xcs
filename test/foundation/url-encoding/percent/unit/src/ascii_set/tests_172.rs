#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_op() {
        let left = AsciiSet::EMPTY.add(b'A');
        let right = AsciiSet::EMPTY.add(b'B');
        let expected = AsciiSet::EMPTY.add(b'A').add(b'B');
        assert_eq!(left + right, expected);
    }

    #[test]
    fn not_op() {
        let set = AsciiSet::EMPTY.add(b'A').add(b'B');
        let not_set = !set;
        assert!(!not_set.contains(b'A'));
        assert!(not_set.contains(b'C'));
    }

    /// This test ensures that we can get the union of two sets as a constant value, which is
    /// useful for defining sets in a modular way.
    #[test]
    fn union() {
        const A: AsciiSet = AsciiSet::EMPTY.add(b'A');
        const B: AsciiSet = AsciiSet::EMPTY.add(b'B');
        const UNION: AsciiSet = A.union(B);
        const EXPECTED: AsciiSet = AsciiSet::EMPTY.add(b'A').add(b'B');
        assert_eq!(UNION, EXPECTED);
    }

    /// This test ensures that we can get the complement of a set as a constant value, which is
    /// useful for defining sets in a modular way.
    #[test]
    fn complement() {
        const BOTH: AsciiSet = AsciiSet::EMPTY.add(b'A').add(b'B');
        const COMPLEMENT: AsciiSet = BOTH.complement();
        assert!(!COMPLEMENT.contains(b'A'));
        assert!(!COMPLEMENT.contains(b'B'));
        assert!(COMPLEMENT.contains(b'C'));
    }
}
