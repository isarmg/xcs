#[cfg(test)]
mod tests {
    use super::*;
    use crate::Locale;

    #[test]
    fn test_data_locale_conversion() {
        #[derive(Debug)]
        struct TestCase<'a> {
            input: &'a str,
            language_priority: &'a str,
            region_priority: &'a str,
        }
        let test_cases = [
            TestCase {
                input: "en",
                language_priority: "en",
                region_priority: "en",
            },
            TestCase {
                input: "en-US",
                language_priority: "en-US",
                region_priority: "en-US",
            },
            TestCase {
                input: "en-u-sd-ustx",
                language_priority: "en-US-u-sd-ustx",
                region_priority: "en-US-u-sd-ustx",
            },
            TestCase {
                input: "en-US-u-sd-ustx",
                language_priority: "en-US-u-sd-ustx",
                region_priority: "en-US-u-sd-ustx",
            },
            TestCase {
                input: "en-u-rg-gbzzzz",
                language_priority: "en",
                region_priority: "en-GB",
            },
            TestCase {
                input: "en-US-u-rg-gbzzzz",
                language_priority: "en-US",
                region_priority: "en-GB",
            },
            TestCase {
                input: "!en-US-u-sd-gbzzzz",
                language_priority: "en-US",
                region_priority: "en-US",
            },
            TestCase {
                input: "en-u-rg-gbzzzz-sd-ustx",
                language_priority: "en-US-u-sd-ustx",
                region_priority: "en-GB",
            },
            TestCase {
                input: "en-US-u-rg-gbzzzz-sd-ustx",
                language_priority: "en-US-u-sd-ustx",
                region_priority: "en-GB",
            },
            TestCase {
                input: "en-US-u-rg-gbeng-sd-ustx",
                language_priority: "en-US-u-sd-ustx",
                region_priority: "en-GB-u-sd-gbeng",
            },
            TestCase {
                input: "!en-TR-u-rg-true",
                language_priority: "en-TR",
                region_priority: "en-TR",
            },
            TestCase {
                input: "!en-US-u-sd-tx",
                language_priority: "en-US",
                region_priority: "en-US",
            },
            TestCase {
                input: "!en-GB-u-rg-tx",
                language_priority: "en-GB",
                region_priority: "en-GB",
            },
            TestCase {
                input: "en-US-u-rg-eng",
                language_priority: "en-US",
                region_priority: "en-EN-u-sd-eng",
            },
            TestCase {
                // All alphabetic values of `-u-sd` are valid, as they are of length 3+, so there's
                // always at least a one-character subdivision. Numeric regions can lead to invalid
                // values though.
                input: "!en-001-u-sd-001",
                language_priority: "en-001",
                region_priority: "en-001",
            },
        ];
        for test_case in test_cases.iter() {
            let prefs = if let Some(locale) = test_case.input.strip_prefix("!") {
                LocalePreferences::from_locale_strict(&Locale::try_from_str(locale).unwrap())
                    .expect_err(locale)
            } else {
                LocalePreferences::from_locale_strict(
                    &Locale::try_from_str(test_case.input).unwrap(),
                )
                .expect(test_case.input)
            };
            assert_eq!(
                prefs.to_data_locale_language_priority().to_string(),
                test_case.language_priority,
                "{test_case:?}"
            );
            assert_eq!(
                prefs.to_data_locale_region_priority().to_string(),
                test_case.region_priority,
                "{test_case:?}"
            );
        }
    }
}
