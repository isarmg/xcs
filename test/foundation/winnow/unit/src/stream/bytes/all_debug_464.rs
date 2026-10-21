#[cfg(all(test, feature = "std"))]
mod debug {
    use crate::stream::Bytes;
    use crate::stream::Stream as _;
    use snapbox::assert_data_eq;
    use snapbox::str;

    #[test]
    fn test_debug() {
        let input = Bytes::new(b"\0\0\0 ftypisom\0\0\x02\0isomiso2avc1mp");
        let expected = str!["000000206674797069736F6D0000020069736F6D69736F32617663316D70"];
        assert_data_eq!(&format!("{input:?}"), expected);
    }

    #[test]
    fn test_pretty_debug() {
        let input = Bytes::new(b"\0\0\0 ftypisom\0\0\x02\0isomiso2avc1mp");
        let expected = str!["000000206674797069736F6D0000020069736F6D69736F32617663316D70"];
        assert_data_eq!(&format!("{input:#?}").replace('_', ""), expected);
    }

    #[test]
    fn test_trace() {
        let input = Bytes::new(b"\0\0\0 ftypisom\0\0\x02\0isomiso2avc1mp");
        let expected = str!["000000206674797069736F6D0000020069736F6D69736F32617663316D70"];
        assert_data_eq!(
            crate::util::from_fn(|f| input.trace(f))
                .to_string()
                .replace('_', ""),
            expected
        );
    }

    #[test]
    fn test_sliced() {
        // Output can change from run-to-run
        let total = Bytes::new(b"12345678901234567890");
        let _ = format!("{total:#?}");
        let _ = format!("{:#?}", &total[1..]);
        let _ = format!("{:#?}", &total[10..]);
    }
}
