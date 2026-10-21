#[cfg(test)]
mod test_char_boundary {
    use super::*;

    #[test]
    fn ascii() {
        let input = "hi";
        let cases = [(0, 0..1), (1, 1..2), (2, 2..2)];
        for (offset, expected) in cases {
            assert_eq!(
                char_boundary(input.as_bytes(), offset),
                expected,
                "input={input:?}, offset={offset:?}"
            );
        }
    }

    #[test]
    fn utf8() {
        let input = "βèƒôřè";
        assert_eq!(input.len(), 12);
        let cases = [
            (0, 0..2),
            (1, 0..2),
            (2, 2..4),
            (3, 2..4),
            (4, 4..6),
            (5, 4..6),
            (6, 6..8),
            (7, 6..8),
            (8, 8..10),
            (9, 8..10),
            (10, 10..12),
            (11, 10..12),
            (12, 12..12),
        ];
        for (offset, expected) in cases {
            assert_eq!(
                char_boundary(input.as_bytes(), offset),
                expected,
                "input={input:?}, offset={offset:?}"
            );
        }
    }
}
