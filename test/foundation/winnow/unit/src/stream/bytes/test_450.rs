#[cfg(test)]
mod test {
    use crate::stream::Bytes;

    #[test]
    fn partial_eq_bytes_byte_slice() {
        let input = b"foo".as_slice();
        let actual = Bytes::new(input);
        assert!(actual == input);
    }

    #[test]
    fn partial_eq_byte_slice_bytes() {
        let input = b"foo".as_slice();
        let actual = Bytes::new(input);
        assert!(input == actual);
    }

    #[test]
    fn partial_eq_bytes_str() {
        let input = "foo";
        let actual = Bytes::new(input);
        assert!(actual == input);
    }

    #[test]
    fn partial_eq_str_bytes() {
        let input = "foo";
        let actual = Bytes::new(input);
        assert!(input == actual);
    }

    #[test]
    fn partial_ord_bytes_byte_slice() {
        let input = b"foo".as_slice();
        let actual = Bytes::new(input);
        assert!(actual.partial_cmp(input) == Some(core::cmp::Ordering::Equal));
    }

    #[test]
    fn partial_ord_byte_slice_bytes() {
        let input = b"foo".as_slice();
        let actual = Bytes::new(input);
        assert!(input.partial_cmp(actual) == Some(core::cmp::Ordering::Equal));
    }

    #[test]
    fn partial_ord_bytes_str() {
        let input = "foo";
        let actual = Bytes::new(input);
        assert!(actual.partial_cmp(input) == Some(core::cmp::Ordering::Equal));
    }

    #[test]
    fn partial_ord_str_bytes() {
        let input = "foo";
        let actual = Bytes::new(input);
        assert!(input.partial_cmp(actual) == Some(core::cmp::Ordering::Equal));
    }
}
