#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use quickcheck::Arbitrary;
    use quickcheck_macros::quickcheck;

    macro_rules! test {
        (
            $name:ident,
            uri = $uri:literal,
            prefix = $prefix:literal,
            expected = $expected:expr,
        ) => {
            #[test]
            fn $name() {
                let uri = $uri.parse().unwrap();
                let new_uri = strip_prefix(&uri, $prefix).map(|uri| uri.to_string());
                assert_eq!(new_uri.as_deref(), $expected);
            }
        };
    }

    test!(empty, uri = "/", prefix = "/", expected = Some("/"),);

    test!(
        single_segment,
        uri = "/a",
        prefix = "/a",
        expected = Some("/"),
    );

    test!(
        single_segment_root_uri,
        uri = "/",
        prefix = "/a",
        expected = None,
    );

    // the prefix is empty, so removing it should have no effect
    test!(
        single_segment_root_prefix,
        uri = "/a",
        prefix = "/",
        expected = Some("/a"),
    );

    test!(
        single_segment_no_match,
        uri = "/a",
        prefix = "/b",
        expected = None,
    );

    test!(
        single_segment_trailing_slash,
        uri = "/a/",
        prefix = "/a/",
        expected = Some("/"),
    );

    test!(
        single_segment_trailing_slash_2,
        uri = "/a",
        prefix = "/a/",
        expected = None,
    );

    test!(
        single_segment_trailing_slash_3,
        uri = "/a/",
        prefix = "/a",
        expected = Some("/"),
    );

    test!(
        multi_segment,
        uri = "/a/b",
        prefix = "/a",
        expected = Some("/b"),
    );

    test!(
        multi_segment_2,
        uri = "/b/a",
        prefix = "/a",
        expected = None,
    );

    test!(
        multi_segment_3,
        uri = "/a",
        prefix = "/a/b",
        expected = None,
    );

    test!(
        multi_segment_4,
        uri = "/a/b",
        prefix = "/b",
        expected = None,
    );

    test!(
        multi_segment_trailing_slash,
        uri = "/a/b/",
        prefix = "/a/b/",
        expected = Some("/"),
    );

    test!(
        multi_segment_trailing_slash_2,
        uri = "/a/b",
        prefix = "/a/b/",
        expected = None,
    );

    test!(
        multi_segment_trailing_slash_3,
        uri = "/a/b/",
        prefix = "/a/b",
        expected = Some("/"),
    );

    test!(
        param_0,
        uri = "/",
        prefix = "/{param}",
        expected = Some("/"),
    );

    test!(
        param_1,
        uri = "/a",
        prefix = "/{param}",
        expected = Some("/"),
    );

    test!(
        param_2,
        uri = "/a/b",
        prefix = "/{param}",
        expected = Some("/b"),
    );

    test!(
        param_3,
        uri = "/b/a",
        prefix = "/{param}",
        expected = Some("/a"),
    );

    test!(
        param_4,
        uri = "/a/b",
        prefix = "/a/{param}",
        expected = Some("/"),
    );

    test!(
        param_5,
        uri = "/b/a",
        prefix = "/a/{param}",
        expected = None,
    );

    test!(
        param_6,
        uri = "/a/b",
        prefix = "/{param}/a",
        expected = None,
    );

    test!(
        param_7,
        uri = "/b/a",
        prefix = "/{param}/a",
        expected = Some("/"),
    );

    test!(
        param_8,
        uri = "/a/b/c",
        prefix = "/a/{param}/c",
        expected = Some("/"),
    );

    test!(
        param_9,
        uri = "/c/b/a",
        prefix = "/a/{param}/c",
        expected = None,
    );

    test!(
        param_10,
        uri = "/a/",
        prefix = "/{param}",
        expected = Some("/"),
    );

    test!(param_11, uri = "/a", prefix = "/{param}/", expected = None,);

    test!(
        param_12,
        uri = "/a/",
        prefix = "/{param}/",
        expected = Some("/"),
    );

    test!(
        param_13,
        uri = "/a/a",
        prefix = "/a/",
        expected = Some("/a"),
    );

    #[quickcheck]
    fn does_not_panic(uri_and_prefix: UriAndPrefix) -> bool {
        let UriAndPrefix { uri, prefix } = uri_and_prefix;
        strip_prefix(&uri, &prefix);
        true
    }

    #[derive(Clone, Debug)]
    struct UriAndPrefix {
        uri: Uri,
        prefix: String,
    }

    impl Arbitrary for UriAndPrefix {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut uri = String::new();
            let mut prefix = String::new();

            let size = u8_between(1, 20, g);

            for _ in 0..size {
                let segment = ascii_alphanumeric(g);

                uri.push('/');
                uri.push_str(&segment);

                prefix.push('/');

                let make_matching_segment = bool::arbitrary(g);
                let make_capture = bool::arbitrary(g);

                match (make_matching_segment, make_capture) {
                    (_, true) => {
                        prefix.push_str(":a");
                    }
                    (true, false) => {
                        prefix.push_str(&segment);
                    }
                    (false, false) => {
                        prefix.push_str(&ascii_alphanumeric(g));
                    }
                }
            }

            if bool::arbitrary(g) {
                uri.push('/');
            }

            if bool::arbitrary(g) {
                prefix.push('/');
            }

            Self {
                uri: uri.parse().unwrap(),
                prefix,
            }
        }
    }

    fn ascii_alphanumeric(g: &mut quickcheck::Gen) -> String {
        #[derive(Clone)]
        struct AsciiAlphanumeric(String);

        impl Arbitrary for AsciiAlphanumeric {
            fn arbitrary(g: &mut quickcheck::Gen) -> Self {
                let mut out = String::new();

                let size = u8_between(1, 20, g) as usize;

                while out.len() < size {
                    let c = char::arbitrary(g);
                    if c.is_ascii_alphanumeric() {
                        out.push(c);
                    }
                }
                Self(out)
            }
        }

        let out = AsciiAlphanumeric::arbitrary(g).0;
        assert!(!out.is_empty());
        out
    }

    fn u8_between(lower: u8, upper: u8, g: &mut quickcheck::Gen) -> u8 {
        loop {
            let size = u8::arbitrary(g);
            if size > lower && size <= upper {
                break size;
            }
        }
    }
}
