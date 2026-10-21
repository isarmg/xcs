#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equals_different_lengths() {
        assert!(!is_equal(b"", b"a"));
        assert!(!is_equal(b"a", b""));
        assert!(!is_equal(b"ab", b"a"));
        assert!(!is_equal(b"a", b"ab"));
    }

    #[test]
    fn equals_mismatch() {
        let one_mismatch = [
            (&b"a"[..], &b"x"[..]),
            (&b"ab"[..], &b"ax"[..]),
            (&b"abc"[..], &b"abx"[..]),
            (&b"abcd"[..], &b"abcx"[..]),
            (&b"abcde"[..], &b"abcdx"[..]),
            (&b"abcdef"[..], &b"abcdex"[..]),
            (&b"abcdefg"[..], &b"abcdefx"[..]),
            (&b"abcdefgh"[..], &b"abcdefgx"[..]),
            (&b"abcdefghi"[..], &b"abcdefghx"[..]),
            (&b"abcdefghij"[..], &b"abcdefghix"[..]),
            (&b"abcdefghijk"[..], &b"abcdefghijx"[..]),
            (&b"abcdefghijkl"[..], &b"abcdefghijkx"[..]),
            (&b"abcdefghijklm"[..], &b"abcdefghijklx"[..]),
            (&b"abcdefghijklmn"[..], &b"abcdefghijklmx"[..]),
        ];
        for (x, y) in one_mismatch {
            assert_eq!(x.len(), y.len(), "lengths should match");
            assert!(!is_equal(x, y));
            assert!(!is_equal(y, x));
        }
    }

    #[test]
    fn equals_yes() {
        assert!(is_equal(b"", b""));
        assert!(is_equal(b"a", b"a"));
        assert!(is_equal(b"ab", b"ab"));
        assert!(is_equal(b"abc", b"abc"));
        assert!(is_equal(b"abcd", b"abcd"));
        assert!(is_equal(b"abcde", b"abcde"));
        assert!(is_equal(b"abcdef", b"abcdef"));
        assert!(is_equal(b"abcdefg", b"abcdefg"));
        assert!(is_equal(b"abcdefgh", b"abcdefgh"));
        assert!(is_equal(b"abcdefghi", b"abcdefghi"));
    }

    #[test]
    fn prefix() {
        assert!(is_prefix(b"", b""));
        assert!(is_prefix(b"a", b""));
        assert!(is_prefix(b"ab", b""));
        assert!(is_prefix(b"foo", b"foo"));
        assert!(is_prefix(b"foobar", b"foo"));

        assert!(!is_prefix(b"foo", b"fob"));
        assert!(!is_prefix(b"foobar", b"fob"));
    }

    #[test]
    fn suffix() {
        assert!(is_suffix(b"", b""));
        assert!(is_suffix(b"a", b""));
        assert!(is_suffix(b"ab", b""));
        assert!(is_suffix(b"foo", b"foo"));
        assert!(is_suffix(b"foobar", b"bar"));

        assert!(!is_suffix(b"foo", b"goo"));
        assert!(!is_suffix(b"foobar", b"gar"));
    }
}
