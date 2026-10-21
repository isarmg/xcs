#[cfg(test)]
mod test {
    use crate::stream::BStr;

    #[test]
    fn partial_eq_bstr_byte_slice() {
        let input = b"foo".as_slice();
        let actual = BStr::new(input);
        assert!(actual == input);
    }

    #[test]
    fn partial_eq_byte_slice_bstr() {
        let input = b"foo".as_slice();
        let actual = BStr::new(input);
        assert!(input == actual);
    }

    #[test]
    fn partial_eq_bstr_str() {
        let input = "foo";
        let actual = BStr::new(input);
        assert!(actual == input);
    }

    #[test]
    fn partial_eq_str_bstr() {
        let input = "foo";
        let actual = BStr::new(input);
        assert!(input == actual);
    }

    #[test]
    fn partial_ord_bstr_byte_slice() {
        let input = b"foo".as_slice();
        let actual = BStr::new(input);
        assert!(actual.partial_cmp(input) == Some(core::cmp::Ordering::Equal));
    }

    #[test]
    fn partial_ord_byte_slice_bstr() {
        let input = b"foo".as_slice();
        let actual = BStr::new(input);
        assert!(input.partial_cmp(actual) == Some(core::cmp::Ordering::Equal));
    }

    #[test]
    fn partial_ord_bstr_str() {
        let input = "foo";
        let actual = BStr::new(input);
        assert!(actual.partial_cmp(input) == Some(core::cmp::Ordering::Equal));
    }

    #[test]
    fn partial_ord_str_bstr() {
        let input = "foo";
        let actual = BStr::new(input);
        assert!(input.partial_cmp(actual) == Some(core::cmp::Ordering::Equal));
    }
}
