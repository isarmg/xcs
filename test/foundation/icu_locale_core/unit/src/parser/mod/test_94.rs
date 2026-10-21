#[cfg(test)]
mod test {
    use super::*;

    fn slice_to_str(input: &[u8]) -> &str {
        std::str::from_utf8(input).unwrap()
    }

    #[test]
    fn subtag_iterator_peek_test() {
        let slice = "de-at-u-ca-foobar";
        let mut si = SubtagIterator::new(slice.as_bytes());

        assert_eq!(si.peek().map(slice_to_str), Some("de"));
        assert_eq!(si.peek().map(slice_to_str), Some("de"));
        assert_eq!(si.next().map(slice_to_str), Some("de"));

        assert_eq!(si.peek().map(slice_to_str), Some("at"));
        assert_eq!(si.peek().map(slice_to_str), Some("at"));
        assert_eq!(si.next().map(slice_to_str), Some("at"));
    }

    #[test]
    fn subtag_iterator_test() {
        let slice = "";
        let mut si = SubtagIterator::new(slice.as_bytes());
        assert_eq!(si.next().map(slice_to_str), Some(""));

        let slice = "-";
        let mut si = SubtagIterator::new(slice.as_bytes());
        assert_eq!(si.next().map(slice_to_str), Some(""));

        let slice = "-en";
        let mut si = SubtagIterator::new(slice.as_bytes());
        assert_eq!(si.next().map(slice_to_str), Some(""));
        assert_eq!(si.next().map(slice_to_str), Some("en"));
        assert_eq!(si.next(), None);

        let slice = "en";
        let si = SubtagIterator::new(slice.as_bytes());
        assert_eq!(si.map(slice_to_str).collect::<Vec<_>>(), vec!["en",]);

        let slice = "en-";
        let si = SubtagIterator::new(slice.as_bytes());
        assert_eq!(si.map(slice_to_str).collect::<Vec<_>>(), vec!["en", "",]);

        let slice = "--";
        let mut si = SubtagIterator::new(slice.as_bytes());
        assert_eq!(si.next().map(slice_to_str), Some(""));
        assert_eq!(si.next().map(slice_to_str), Some(""));
        assert_eq!(si.next().map(slice_to_str), Some(""));
        assert_eq!(si.next(), None);

        let slice = "-en-";
        let mut si = SubtagIterator::new(slice.as_bytes());
        assert_eq!(si.next().map(slice_to_str), Some(""));
        assert_eq!(si.next().map(slice_to_str), Some("en"));
        assert_eq!(si.next().map(slice_to_str), Some(""));
        assert_eq!(si.next(), None);

        let slice = "de-at-u-ca-foobar";
        let si = SubtagIterator::new(slice.as_bytes());
        assert_eq!(
            si.map(slice_to_str).collect::<Vec<_>>(),
            vec!["de", "at", "u", "ca", "foobar",]
        );
    }

    #[test]
    fn skip_before_separator_test() {
        let current = skip_before_separator(b"");
        assert_eq!(current, b"");

        let current = skip_before_separator(b"en");
        assert_eq!(current, b"en");

        let current = skip_before_separator(b"en-");
        assert_eq!(current, b"en");

        let current = skip_before_separator(b"en--US");
        assert_eq!(current, b"en");

        let current = skip_before_separator(b"-US");
        assert_eq!(current, b"");

        let current = skip_before_separator(b"US");
        assert_eq!(current, b"US");

        let current = skip_before_separator(b"-");
        assert_eq!(current, b"");
    }
}
