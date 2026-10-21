#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_type_() {
        assert_eq!(TEXT_PLAIN.type_(), TEXT);
    }

    #[test]
    fn test_subtype() {
        assert_eq!(TEXT_PLAIN.subtype(), PLAIN);
        assert_eq!(TEXT_PLAIN_UTF_8.subtype(), PLAIN);
        let mime = Mime::from_str("text/html+xml").unwrap();
        assert_eq!(mime.subtype(), HTML);
    }

    #[test]
    fn test_matching() {
        match (TEXT_PLAIN.type_(), TEXT_PLAIN.subtype()) {
            (TEXT, PLAIN) => (),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_suffix() {
        assert_eq!(TEXT_PLAIN.suffix(), None);
        let mime = Mime::from_str("text/html+xml").unwrap();
        assert_eq!(mime.suffix(), Some(XML));
    }

    #[test]
    fn test_mime_fmt() {
        let mime = TEXT_PLAIN;
        assert_eq!(mime.to_string(), "text/plain");
        let mime = TEXT_PLAIN_UTF_8;
        assert_eq!(mime.to_string(), "text/plain; charset=utf-8");
    }

    #[test]
    fn test_mime_from_str() {
        assert_eq!(Mime::from_str("text/plain").unwrap(), TEXT_PLAIN);
        assert_eq!(Mime::from_str("TEXT/PLAIN").unwrap(), TEXT_PLAIN);
        assert_eq!(
            Mime::from_str("text/plain;charset=utf-8").unwrap(),
            TEXT_PLAIN_UTF_8
        );
        assert_eq!(
            Mime::from_str("text/plain;charset=\"utf-8\"").unwrap(),
            TEXT_PLAIN_UTF_8
        );

        // spaces
        assert_eq!(
            Mime::from_str("text/plain; charset=utf-8").unwrap(),
            TEXT_PLAIN_UTF_8
        );

        // quotes + semi colon
        Mime::from_str("text/plain;charset=\"utf-8\"; foo=bar").unwrap();
        Mime::from_str("text/plain;charset=\"utf-8\" ; foo=bar").unwrap();

        let upper = Mime::from_str("TEXT/PLAIN").unwrap();
        assert_eq!(upper, TEXT_PLAIN);
        assert_eq!(upper.type_(), TEXT);
        assert_eq!(upper.subtype(), PLAIN);

        let extended = Mime::from_str("TEXT/PLAIN; CHARSET=UTF-8; FOO=BAR").unwrap();
        assert_eq!(extended, "text/plain; charset=utf-8; foo=BAR");
        assert_eq!(extended.get_param("charset").unwrap(), "utf-8");
        assert_eq!(extended.get_param("foo").unwrap(), "BAR");

        Mime::from_str("multipart/form-data; boundary=--------foobar").unwrap();

        // stars
        assert_eq!("*/*".parse::<Mime>().unwrap(), STAR_STAR);
        assert_eq!("image/*".parse::<Mime>().unwrap(), "image/*");
        assert_eq!(
            "text/*; charset=utf-8".parse::<Mime>().unwrap(),
            "text/*; charset=utf-8"
        );

        // parse errors
        Mime::from_str("f o o / bar").unwrap_err();
        Mime::from_str("text\n/plain").unwrap_err();
        Mime::from_str("text\r/plain").unwrap_err();
        Mime::from_str("text/\r\nplain").unwrap_err();
        Mime::from_str("text/plain;\r\ncharset=utf-8").unwrap_err();
        Mime::from_str("text/plain; charset=\r\nutf-8").unwrap_err();
        Mime::from_str("text/plain; charset=\"\r\nutf-8\"").unwrap_err();
    }

    #[test]
    fn test_mime_from_str_empty_parameter_list() {
        static CASES: &'static [&'static str] = &[
            "text/event-stream;",
            "text/event-stream; ",
            "text/event-stream;       ",
        ];

        for case in CASES {
            let mime = Mime::from_str(case).expect(case);
            assert_eq!(mime, TEXT_EVENT_STREAM, "case = {:?}", case);
            assert_eq!(mime.type_(), TEXT, "case = {:?}", case);
            assert_eq!(mime.subtype(), EVENT_STREAM, "case = {:?}", case);
            assert!(!mime.has_params(), "case = {:?}", case);
        }
    }

    #[test]
    fn test_case_sensitive_values() {
        let mime = Mime::from_str("multipart/form-data; charset=BASE64; boundary=ABCDEFG").unwrap();
        assert_eq!(mime.get_param(CHARSET).unwrap(), "bAsE64");
        assert_eq!(mime.get_param(BOUNDARY).unwrap(), "ABCDEFG");
        assert_ne!(mime.get_param(BOUNDARY).unwrap(), "abcdefg");
    }

    #[test]
    fn test_get_param() {
        assert_eq!(TEXT_PLAIN.get_param("charset"), None);
        assert_eq!(TEXT_PLAIN.get_param("baz"), None);

        assert_eq!(TEXT_PLAIN_UTF_8.get_param("charset"), Some(UTF_8));
        assert_eq!(TEXT_PLAIN_UTF_8.get_param("baz"), None);

        let mime = Mime::from_str("text/plain; charset=utf-8; foo=bar").unwrap();
        assert_eq!(mime.get_param(CHARSET).unwrap(), "utf-8");
        assert_eq!(mime.get_param("foo").unwrap(), "bar");
        assert_eq!(mime.get_param("baz"), None);

        let mime = Mime::from_str("text/plain;charset=\"utf-8\"").unwrap();
        assert_eq!(mime.get_param(CHARSET), Some(UTF_8));
    }

    #[test]
    fn test_name_eq() {
        assert_eq!(TEXT, TEXT);
        assert_eq!(TEXT, "text");
        assert_eq!("text", TEXT);
        assert_eq!(TEXT, "TEXT");

        let param = Name {
            source: "ABC",
            insensitive: false,
        };

        assert_eq!(param, param);
        assert_eq!(param, "ABC");
        assert_eq!("ABC", param);
        assert_ne!(param, "abc");
        assert_ne!("abc", param);
    }

    #[test]
    fn test_essence_str() {
        assert_eq!(TEXT_PLAIN.essence_str(), "text/plain");
        assert_eq!(TEXT_PLAIN_UTF_8.essence_str(), "text/plain");
        assert_eq!(IMAGE_SVG.essence_str(), "image/svg+xml");
    }
}
