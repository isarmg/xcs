#[cfg(test)]
mod tests {
    use super::*;

    fn r(range: RangeInclusive<u8>) -> Utf8Range {
        Utf8Range { start: *range.start(), end: *range.end() }
    }

    fn split_maybe(
        old: RangeInclusive<u8>,
        new: RangeInclusive<u8>,
    ) -> Option<Split> {
        Split::new(r(old), r(new))
    }

    fn split(
        old: RangeInclusive<u8>,
        new: RangeInclusive<u8>,
    ) -> Vec<SplitRange> {
        split_maybe(old, new).unwrap().as_slice().to_vec()
    }

    #[test]
    fn no_splits() {
        // case 1
        assert_eq!(None, split_maybe(0..=1, 2..=3));
        // case 2
        assert_eq!(None, split_maybe(2..=3, 0..=1));
    }

    #[test]
    fn splits() {
        let range = |r: RangeInclusive<u8>| Utf8Range {
            start: *r.start(),
            end: *r.end(),
        };
        let old = |r| SplitRange::Old(range(r));
        let new = |r| SplitRange::New(range(r));
        let both = |r| SplitRange::Both(range(r));

        // case 3
        assert_eq!(split(0..=0, 0..=0), vec![both(0..=0)]);
        assert_eq!(split(9..=9, 9..=9), vec![both(9..=9)]);

        // case 4
        assert_eq!(split(0..=5, 0..=6), vec![both(0..=5), new(6..=6)]);
        assert_eq!(split(0..=5, 0..=8), vec![both(0..=5), new(6..=8)]);
        assert_eq!(split(5..=5, 5..=8), vec![both(5..=5), new(6..=8)]);

        // case 5
        assert_eq!(split(1..=5, 0..=5), vec![new(0..=0), both(1..=5)]);
        assert_eq!(split(3..=5, 0..=5), vec![new(0..=2), both(3..=5)]);
        assert_eq!(split(5..=5, 0..=5), vec![new(0..=4), both(5..=5)]);

        // case 6
        assert_eq!(split(0..=6, 0..=5), vec![both(0..=5), old(6..=6)]);
        assert_eq!(split(0..=8, 0..=5), vec![both(0..=5), old(6..=8)]);
        assert_eq!(split(5..=8, 5..=5), vec![both(5..=5), old(6..=8)]);

        // case 7
        assert_eq!(split(0..=5, 1..=5), vec![old(0..=0), both(1..=5)]);
        assert_eq!(split(0..=5, 3..=5), vec![old(0..=2), both(3..=5)]);
        assert_eq!(split(0..=5, 5..=5), vec![old(0..=4), both(5..=5)]);

        // case 8
        assert_eq!(
            split(3..=6, 2..=7),
            vec![new(2..=2), both(3..=6), new(7..=7)],
        );
        assert_eq!(
            split(3..=6, 1..=8),
            vec![new(1..=2), both(3..=6), new(7..=8)],
        );

        // case 9
        assert_eq!(
            split(2..=7, 3..=6),
            vec![old(2..=2), both(3..=6), old(7..=7)],
        );
        assert_eq!(
            split(1..=8, 3..=6),
            vec![old(1..=2), both(3..=6), old(7..=8)],
        );

        // case 10
        assert_eq!(
            split(3..=6, 6..=7),
            vec![old(3..=5), both(6..=6), new(7..=7)],
        );
        assert_eq!(
            split(3..=6, 6..=8),
            vec![old(3..=5), both(6..=6), new(7..=8)],
        );
        assert_eq!(
            split(5..=6, 6..=7),
            vec![old(5..=5), both(6..=6), new(7..=7)],
        );

        // case 11
        assert_eq!(
            split(6..=7, 3..=6),
            vec![new(3..=5), both(6..=6), old(7..=7)],
        );
        assert_eq!(
            split(6..=8, 3..=6),
            vec![new(3..=5), both(6..=6), old(7..=8)],
        );
        assert_eq!(
            split(6..=7, 5..=6),
            vec![new(5..=5), both(6..=6), old(7..=7)],
        );

        // case 12
        assert_eq!(
            split(3..=7, 5..=9),
            vec![old(3..=4), both(5..=7), new(8..=9)],
        );
        assert_eq!(
            split(3..=5, 4..=6),
            vec![old(3..=3), both(4..=5), new(6..=6)],
        );

        // case 13
        assert_eq!(
            split(5..=9, 3..=7),
            vec![new(3..=4), both(5..=7), old(8..=9)],
        );
        assert_eq!(
            split(4..=6, 3..=5),
            vec![new(3..=3), both(4..=5), old(6..=6)],
        );
    }

    // Arguably there should be more tests here, but in practice, this data
    // structure is well covered by the huge number of regex tests.
}
