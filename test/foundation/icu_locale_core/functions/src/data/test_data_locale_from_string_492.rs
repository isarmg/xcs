#[test]
fn test_data_locale_from_string() {
    #[derive(Debug)]
    struct TestCase {
        pub input: &'static str,
        pub success: bool,
    }

    for cas in [
        TestCase {
            input: "und",
            success: true,
        },
        TestCase {
            input: "und-u-cu-gbp",
            success: false,
        },
        TestCase {
            input: "en-ZA-u-sd-zaa",
            success: true,
        },
        TestCase {
            input: "en...",
            success: false,
        },
    ] {
        let data_locale = match (DataLocale::from_str(cas.input), cas.success) {
            (Ok(l), true) => l,
            (Err(_), false) => {
                continue;
            }
            (Ok(_), false) => {
                panic!("DataLocale parsed but it was supposed to fail: {cas:?}");
            }
            (Err(_), true) => {
                panic!("DataLocale was supposed to parse but it failed: {cas:?}");
            }
        };
        writeable::assert_writeable_eq!(data_locale, cas.input);
    }
}
