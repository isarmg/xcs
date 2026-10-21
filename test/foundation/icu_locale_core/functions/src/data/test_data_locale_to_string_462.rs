#[test]
fn test_data_locale_to_string() {
    struct TestCase {
        pub locale: &'static str,
        pub expected: &'static str,
    }

    for cas in [
        TestCase {
            locale: "und",
            expected: "und",
        },
        TestCase {
            locale: "und-u-sd-sdd",
            expected: "und-SD-u-sd-sdd",
        },
        TestCase {
            locale: "en-ZA-u-sd-zaa",
            expected: "en-ZA-u-sd-zaa",
        },
        TestCase {
            locale: "en-ZA-u-sd-sdd",
            expected: "en-ZA",
        },
    ] {
        let locale = cas.locale.parse::<DataLocale>().unwrap();
        writeable::assert_writeable_eq!(locale, cas.expected);
    }
}
