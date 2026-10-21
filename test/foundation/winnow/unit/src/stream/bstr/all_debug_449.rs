#[cfg(all(test, feature = "std"))]
mod debug {
    use crate::stream::BStr;
    use crate::stream::Stream as _;
    use snapbox::assert_data_eq;
    use snapbox::str;

    #[test]
    fn test_debug() {
        let input = BStr::new(b"abc");
        let expected = str![[r#""abc""#]];
        assert_data_eq!(&format!("{input:?}"), expected);

        let input = BStr::new(b"\0\0\0 ftypisom\0\0\x02\0isomiso2avc1mp");
        let expected = str![[r#""/0/0/0 ftypisom/0/0/u{2}/0isomiso2avc1mp""#]];
        assert_data_eq!(&format!("{input:?}"), expected);
    }

    #[test]
    fn test_pretty_debug() {
        let input = BStr::new(b"abc");
        let expected = str!["abc"];
        assert_data_eq!(&format!("{input:#?}"), expected);
    }

    #[test]
    fn test_trace() {
        let input = BStr::new(b"abc");
        let expected = str!["abc"];
        assert_data_eq!(
            crate::util::from_fn(|f| input.trace(f)).to_string(),
            expected
        );
    }
}
