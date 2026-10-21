#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn percent_encode_byte() {
        for i in 0..=0xFF {
            let encoded = super::percent_encode_byte(i);
            assert_eq!(encoded, alloc::format!("%{:02X}", i));
        }
    }

    #[test]
    fn percent_encode_accepts_ascii_set_ref() {
        let encoded = percent_encode(b"foo bar?", &AsciiSet::EMPTY);
        assert_eq!(encoded.collect::<String>(), "foo bar?");
    }

    #[test]
    fn percent_encode_collect() {
        let encoded = percent_encode(b"foo bar?", NON_ALPHANUMERIC);
        assert_eq!(encoded.collect::<String>(), String::from("foo%20bar%3F"));

        let encoded = percent_encode(b"\x00\x01\x02\x03", CONTROLS);
        assert_eq!(encoded.collect::<String>(), String::from("%00%01%02%03"));
    }

    #[test]
    fn percent_encode_display() {
        let encoded = percent_encode(b"foo bar?", NON_ALPHANUMERIC);
        assert_eq!(alloc::format!("{}", encoded), "foo%20bar%3F");
    }

    #[test]
    fn percent_encode_cow() {
        let encoded = percent_encode(b"foo bar?", NON_ALPHANUMERIC);
        assert_eq!(Cow::from(encoded), "foo%20bar%3F");
    }

    #[test]
    fn utf8_percent_encode_accepts_ascii_set_ref() {
        let encoded = super::utf8_percent_encode("foo bar?", &AsciiSet::EMPTY);
        assert_eq!(encoded.collect::<String>(), "foo bar?");
    }

    #[test]
    fn utf8_percent_encode() {
        assert_eq!(
            super::utf8_percent_encode("foo bar?", NON_ALPHANUMERIC),
            percent_encode(b"foo bar?", NON_ALPHANUMERIC)
        );
    }

    #[test]
    fn percent_decode() {
        assert_eq!(
            super::percent_decode(b"foo%20bar%3f")
                .decode_utf8()
                .unwrap(),
            "foo bar?"
        );
    }

    #[test]
    fn percent_decode_str() {
        assert_eq!(
            super::percent_decode_str("foo%20bar%3f")
                .decode_utf8()
                .unwrap(),
            "foo bar?"
        );
    }

    #[test]
    fn percent_decode_collect() {
        let decoded = super::percent_decode(b"foo%20bar%3f");
        assert_eq!(decoded.collect::<Vec<u8>>(), b"foo bar?");
    }

    #[test]
    fn percent_decode_cow() {
        let decoded = super::percent_decode(b"foo%20bar%3f");
        assert_eq!(Cow::from(decoded), Cow::Owned::<[u8]>(b"foo bar?".to_vec()));

        let decoded = super::percent_decode(b"foo bar?");
        assert_eq!(Cow::from(decoded), Cow::Borrowed(b"foo bar?"));
    }

    #[test]
    fn percent_decode_invalid_utf8() {
        // Invalid UTF-8 sequence
        let decoded = super::percent_decode(b"%00%9F%92%96")
            .decode_utf8()
            .unwrap_err();
        assert_eq!(decoded.valid_up_to(), 1);
        assert_eq!(decoded.error_len(), Some(1));
    }

    #[test]
    fn percent_decode_utf8_lossy() {
        assert_eq!(
            super::percent_decode(b"%F0%9F%92%96").decode_utf8_lossy(),
            "💖"
        );
    }

    #[test]
    fn percent_decode_utf8_lossy_invalid_utf8() {
        assert_eq!(
            super::percent_decode(b"%00%9F%92%96").decode_utf8_lossy(),
            "\u{0}���"
        );
    }
}
