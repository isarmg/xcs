#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_custom_section() {
        let custom = CustomSection {
            name: "test".into(),
            data: Cow::Borrowed(&[11, 22, 33, 44]),
        };

        let mut encoded = vec![];
        custom.encode(&mut encoded);

        #[rustfmt::skip]
        assert_eq!(encoded, vec![
            // LEB128 length of section.
            9,
            // LEB128 length of name.
            4,
            // Name.
            b't', b'e', b's', b't',
            // Data.
            11, 22, 33, 44,
        ]);
    }
}
