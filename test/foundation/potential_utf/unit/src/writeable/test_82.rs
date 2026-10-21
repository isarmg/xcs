#[cfg(test)]
mod test {
    #![allow(invalid_from_utf8)] // only way to construct the error
    use super::*;
    use writeable::assert_try_writeable_parts_eq;

    #[test]
    fn test_utf8() {
        assert_try_writeable_parts_eq!(
            PotentialUtf8::from_bytes(b"Foo Bar"),
            "Foo Bar",
            Ok(()),
            []
        );
        assert_try_writeable_parts_eq!(
            PotentialUtf8::from_bytes(b"Foo\xFDBar"),
            "Foo�Bar",
            Err(core::str::from_utf8(b"Foo\xFDBar").unwrap_err()),
            [(3, 6, Part::ERROR)]
        );
        assert_try_writeable_parts_eq!(
            PotentialUtf8::from_bytes(b"Foo\xFDBar\xff"),
            "Foo�Bar�",
            Err(core::str::from_utf8(b"Foo\xFDBar\xff").unwrap_err()),
            [(3, 6, Part::ERROR), (9, 12, Part::ERROR)],
        );
    }

    #[test]
    fn test_utf16() {
        assert_try_writeable_parts_eq!(
            PotentialUtf16::from_slice(&[0xD83E, 0xDD73]),
            "🥳",
            Ok(()),
            []
        );
        assert_try_writeable_parts_eq!(
            PotentialUtf16::from_slice(&[0xD83E, 0x20, 0xDD73]),
            "� �",
            Err(char::decode_utf16([0xD83E].into_iter())
                .next()
                .unwrap()
                .unwrap_err()),
            [(0, 3, Part::ERROR), (4, 7, Part::ERROR)]
        );
    }
}
