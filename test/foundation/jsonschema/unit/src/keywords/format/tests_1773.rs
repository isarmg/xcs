#[cfg(test)]
mod tests {
    use referencing::Draft;
    use json_codec::json;
    use test_case::test_case;

    use crate::{tests_util, EmailOptions};

    use super::*;

    // The full `fluent_uri` parse is the oracle for the byte-level recognizer.
    #[cfg(not(target_arch = "wasm32"))]
    fn agrees_with_fluent_uri(text: &str) {
        assert_eq!(
            is_valid_uri_reference(text),
            referencing::UriRef::parse(text).is_ok(),
            "uri-reference: {text:?}"
        );
        assert_eq!(
            is_valid_uri(text),
            referencing::Uri::parse(text).is_ok(),
            "uri: {text:?}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[hegel::test(test_cases = 20_000)]
    fn uri_syntax_agrees_with_fluent_uri_on_any_text(tc: hegel::TestCase) {
        let text: String = tc.draw(
            hegel::generators::text()
                .alphabet("aZ09:/?#[]@!$&'()*+,;=%-._~ Fé")
                .max_size(24),
        );
        agrees_with_fluent_uri(&text);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[hegel::test(test_cases = 20_000)]
    fn uri_syntax_agrees_with_fluent_uri_on_uri_shapes(tc: hegel::TestCase) {
        let gs = hegel::generators::sampled_from;
        let parts = [
            tc.draw(gs(vec!["", "http:", "a+b-c.d:", "1a:", "a_b:", ":", "A:"])),
            tc.draw(gs(vec!["", "//", "///", "/", "./", "../", ":"])),
            tc.draw(gs(vec!["", "user@", "u:p@", "@", "a@b@", "%41@", "%4@"])),
            tc.draw(gs(vec![
                "",
                "host",
                "127.0.0.1",
                "[::1]",
                "[v1.x]",
                "h%2F",
                "h%zz",
                "a:b",
                "é",
            ])),
            tc.draw(gs(vec!["", ":", ":80", ":8a", ":65536", "::"])),
            tc.draw(gs(vec![
                "", "/", "/a/b", "a:b/c", "/a:b", "//x", "/%20", "/%", "/a b", "/[x]",
            ])),
            tc.draw(gs(vec!["", "?", "?q=1&r", "?a?b/c", "?%zz", "?#"])),
            tc.draw(gs(vec!["", "#", "#f", "#a/b?c", "#a#b", "#%41"])),
        ];
        agrees_with_fluent_uri(&parts.concat());
    }

    #[test_case(b"00" => Some(0);  "min")]
    #[test_case(b"09" => Some(9);  "nine")]
    #[test_case(b"10" => Some(10); "ten")]
    #[test_case(b"59" => Some(59); "fifty-nine")]
    #[test_case(b"99" => Some(99); "max")]
    #[test_case(b"1a" => None; "alpha")]
    #[test_case(b" 5" => None; "leading space")]
    #[test_case(b":0" => None; "colon is not a digit")]
    #[test_case(b";9" => None; "semicolon is not a digit")]
    fn test_parse_two_digits(bytes: &[u8]) -> Option<u8> {
        parse_two_digits(bytes)
    }

    #[test_case(b"0000" => Some(0);    "zero")]
    #[test_case(b"1970" => Some(1970); "epoch year")]
    #[test_case(b"2023" => Some(2023); "recent year")]
    #[test_case(b"9999" => Some(9999); "max")]
    #[test_case(b"199x" => None; "trailing alpha")]
    #[test_case(b" 999" => None; "leading space")]
    #[test_case(b"20:3" => None; "colon is not a digit")]
    fn test_parse_four_digits(bytes: &[u8]) -> Option<u16> {
        parse_four_digits(bytes)
    }

    // Sample strings (A) through (S) of RFC 3492, section 7.1:
    // https://www.rfc-editor.org/rfc/rfc3492#section-7.1
    #[test_case("egbpdaj6bu4bxfgehfvwxn" => Some("ليهمابتكلموشعربي؟".to_string()); "rfc a arabic")]
    #[test_case("ihqwcrb4cv8a8dqg056pqjye" => Some("他们为什么不说中文".to_string()); "rfc b chinese simplified")]
    #[test_case("ihqwctvzc91f659drss3x8bo0yb" => Some("他們爲什麽不說中文".to_string()); "rfc c chinese traditional")]
    #[test_case("Proprostnemluvesky-uyb24dma41a" => Some("Pročprostěnemluvíčesky".to_string()); "rfc d czech")]
    #[test_case("4dbcagdahymbxekheh6e0a7fei0b" => Some("למההםפשוטלאמדבריםעברית".to_string()); "rfc e hebrew")]
    #[test_case("i1baa7eci9glrd9b2ae1bj0hfcgg6iyaf8o0a1dig0cd" => Some("यहलोगहिन्दीक्योंनहींबोलसकतेहैं".to_string()); "rfc f hindi")]
    #[test_case("n8jok5ay5dzabd5bym9f0cm5685rrjetr6pdxa" => Some("なぜみんな日本語を話してくれないのか".to_string()); "rfc g japanese")]
    #[test_case("989aomsvi5e83db1d2a355cv1e0vak1dwrv93d5xbh15a0dt30a5jpsd879ccm6fea98c" => Some("세계의모든사람들이한국어를이해한다면얼마나좋을까".to_string()); "rfc h korean")]
    #[test_case("b1abfaaepdrnnbgefbaDotcwatmq2g4l" => Some("почемужеонинеговорятпорусски".to_string()); "rfc i russian")]
    #[test_case("PorqunopuedensimplementehablarenEspaol-fmd56a" => Some("PorquénopuedensimplementehablarenEspañol".to_string()); "rfc j spanish")]
    #[test_case("TisaohkhngthchnitingVit-kjcr8268qyxafd2f1b9g" => Some("TạisaohọkhôngthểchỉnóitiếngViệt".to_string()); "rfc k vietnamese")]
    #[test_case("3B-ww4c5e180e575a65lsy2b" => Some("3年B組金八先生".to_string()); "rfc l")]
    #[test_case("-with-SUPER-MONKEYS-pc58ag80a8qai00g7n9n" => Some("安室奈美恵-with-SUPER-MONKEYS".to_string()); "rfc m leading hyphen in basic part")]
    #[test_case("Hello-Another-Way--fc4qua05auwb3674vfr0b" => Some("Hello-Another-Way-それぞれの場所".to_string()); "rfc n double hyphen")]
    #[test_case("2-u9tlzr9756bt3uc0v" => Some("ひとつ屋根の下2".to_string()); "rfc o")]
    #[test_case("MajiKoi5-783gue6qz075azm5e" => Some("MajiでKoiする5秒前".to_string()); "rfc p")]
    #[test_case("de-jg4avhby1noc0d" => Some("パフィーdeルンバ".to_string()); "rfc q")]
    #[test_case("d9juau41awczczp" => Some("そのスピードで".to_string()); "rfc r")]
    #[test_case("-> $1.00 <--" => Some("-> $1.00 <-".to_string()); "rfc s ascii only")]
    // Boundaries the sample strings do not reach.
    #[test_case("abc-" => Some("abc".to_string()); "empty extended part")]
    #[test_case("" => Some(String::new()); "empty")]
    #[test_case("a" => Some("\u{80}".to_string()); "no basic code points")]
    #[test_case("99999999999999999999" => None; "overflow")]
    #[test_case("-t7g" => None; "leading delimiter is not a separator")]
    #[test_case("é" => None; "non-ascii")]
    #[test_case("ss" => None; "incomplete")]
    fn test_decode_punycode(input: &str) -> Option<String> {
        decode_punycode(input)
    }

    #[cfg(feature = "idna")]
    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn differential_against_idna() {
        let alphabet: Vec<u8> = (0x20u8..=0x7e).collect();
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut checked = 0u32;
        for _ in 0..400_000 {
            let len = (next() % 40) as usize;
            let candidate: String = (0..len)
                .map(|_| char::from(alphabet[(next() % alphabet.len() as u64) as usize]))
                .collect();
            let ours = decode_punycode(&candidate);
            let theirs = url_encoding::domain::punycode::decode_to_string(&candidate);
            assert_eq!(ours, theirs, "diverged on {candidate:?}");
            checked += 1;
        }
        assert_eq!(checked, 400_000);
    }

    #[test]
    fn ignored_format() {
        let schema = json!({"format": "custom", "type": "string"});
        let instance = json!("foo");
        let validator = crate::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&instance));
    }

    #[test]
    fn format_validation() {
        let schema = json!({"format": "email", "type": "string"});
        let email_instance = json!("email@example.com");
        let not_email_instance = json!("foo");

        let with_validation = crate::options()
            .should_validate_formats(true)
            .build(&schema)
            .unwrap();
        let without_validation = crate::options()
            .should_validate_formats(false)
            .build(&schema)
            .unwrap();

        assert!(with_validation.is_valid(&email_instance));
        assert!(!with_validation.is_valid(&not_email_instance));
        assert!(without_validation.is_valid(&email_instance));
        assert!(without_validation.is_valid(&not_email_instance));
    }

    #[test]
    fn ecma_regex() {
        // See GH-230
        let schema = json!({"format": "regex", "type": "string"});
        let instance = json!("^\\cc$");
        let validator = crate::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&instance));
    }

    /// Each draft, its meta-schema, and whether that meta-schema asserts `format`.
    const DRAFTS: [(Draft, &str, bool); 5] = [
        (
            Draft::Draft4,
            "http://json-schema.org/draft-04/schema#",
            true,
        ),
        (
            Draft::Draft6,
            "http://json-schema.org/draft-06/schema#",
            true,
        ),
        (
            Draft::Draft7,
            "http://json-schema.org/draft-07/schema#",
            true,
        ),
        (
            Draft::Draft201909,
            "https://json-schema.org/draft/2019-09/schema",
            false,
        ),
        (
            Draft::Draft202012,
            "https://json-schema.org/draft/2020-12/schema",
            false,
        ),
    ];

    #[test_case(r"^\-?\d+$", true; "escaped dash")]
    #[test_case(r"^(\-?\d+(\.\d+)?),\s*(\-?\d+(\.\d+)?)$", true; "escaped dashes in groups")]
    #[test_case(r"\/", true; "escaped slash")]
    #[test_case(r"\:", true; "escaped colon")]
    #[test_case(r"\ ", true; "escaped space")]
    #[test_case(r"\@", true; "escaped at sign")]
    #[test_case(r"[\:\@]", true; "escaped punctuation in class")]
    #[test_case(r"[a-z&&^b]", true; "class intersection syntax is literal")]
    #[test_case(r"[a-z--b]", true; "adjacent ranges with doubled hyphen")]
    #[test_case(r"[a~~b]", true; "class symmetric difference syntax is literal")]
    #[test_case(r"[[a]]", false; "lone closing bracket after a nested class opener")]
    #[test_case(r"\p{Letter}", true; "property escape")]
    #[test_case(r"(", false; "unbalanced paren")]
    #[test_case(r"[", false; "unbalanced bracket")]
    #[test_case("\\", false; "trailing backslash")]
    #[test_case(r"\a", false; "escaped letter")]
    #[test_case(r"\u{110000}", false; "out of range braced unicode escape")]
    fn regex_format_per_draft(pattern: &str, expected: bool) {
        for (draft, uri, asserts_format) in DRAFTS {
            let validator = crate::options()
                .with_draft(draft)
                .should_validate_formats(true)
                .build(&json!({"format": "regex"}))
                .expect("Invalid schema");
            assert_eq!(validator.is_valid(&json!(pattern)), expected, "{draft:?}");
            let schema = json!({"$schema": uri, "pattern": pattern});
            assert_eq!(
                crate::meta::is_valid(&schema),
                expected || !asserts_format,
                "{draft:?}"
            );
        }
    }

    #[test_case(r"^\-?\d+$"; "regex engine")]
    #[test_case(r"^(?=\-?\d)\-?\d+$"; "fancy regex engine")]
    fn draft4_pattern_with_escaped_dash(pattern: &str) {
        let schema =
            json!({"$schema": "http://json-schema.org/draft-04/schema#", "pattern": pattern});
        assert!(crate::meta::is_valid(&schema));
        let validator = crate::validator_for(&schema).expect("Invalid schema");
        assert!(validator.is_valid(&json!("-12")));
        assert!(validator.is_valid(&json!("12")));
        assert!(!validator.is_valid(&json!("1-2")));
    }

    #[test]
    fn location() {
        tests_util::assert_schema_location(&json!({"format": "date"}), &json!("bla"), "/format");
    }

    #[test]
    fn uuid() {
        let schema = json!({"format": "uuid", "type": "string"});

        let passing_instance = json!("f308a72c-fa84-11eb-9a03-0242ac130003");
        let failing_instance = json!("1");

        let validator = crate::options()
            .with_draft(Draft::Draft201909)
            .should_validate_formats(true)
            .build(&schema)
            .unwrap();

        assert!(validator.is_valid(&passing_instance));
        assert!(!validator.is_valid(&failing_instance));
    }

    #[test]
    fn uri() {
        let schema = json!({"format": "uri", "type": "string"});

        let passing_instance = json!("https://phillip.com");
        let failing_instance = json!("redis");

        tests_util::is_valid(&schema, &passing_instance);
        tests_util::is_not_valid(&schema, &failing_instance);
    }

    #[test_case("P1Y1Y")]
    #[test_case("PT1H1H")]
    #[test_case("P1WT1H")]
    fn test_invalid_duration(input: &str) {
        assert!(!is_valid_duration(input));
    }

    #[cfg(not(feature = "idna"))]
    #[test_case("idn-hostname")]
    #[test_case("idn-email")]
    fn idn_formats_are_unknown_without_idna(format: &str) {
        let schema = json!({"format": format, "type": "string"});
        let validator = crate::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("a valid schema");
        assert!(validator.is_valid(&json!("anything")));

        let error = crate::options()
            .should_validate_formats(true)
            .should_ignore_unknown_formats(false)
            .build(&schema)
            .expect_err("the validation error should be returned");
        assert_eq!(
            error.to_string(),
            format!(
                "Unknown format: '{format}'. Adjust configuration to ignore unrecognized formats"
            )
        );
    }

    #[cfg(feature = "idna")]
    #[test_case(
        "idn-hostname",
        "\u{5B9F}\u{4F8B}.\u{30C6}\u{30B9}\u{30C8}",
        "-\u{5B9F}\u{4F8B}"
    )]
    #[test_case("idn-email", "\u{03B1}@\u{03C0}\u{03B1}\u{03C1}.gr", "a@-.gr")]
    fn idn_formats_validate_with_idna(format: &str, accepted: &str, rejected: &str) {
        let schema = json!({"format": format, "type": "string"});
        let validator = crate::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("a valid schema");
        assert!(validator.is_valid(&json!(accepted)));
        assert!(!validator.is_valid(&json!(rejected)));
    }

    #[test]
    fn unknown_formats_should_not_be_ignored() {
        let schema = json!({ "format": "custom", "type": "string"});
        let error = crate::options()
            .should_validate_formats(true)
            .should_ignore_unknown_formats(false)
            .build(&schema)
            .expect_err("the validation error should be returned");

        assert_eq!(
            error.to_string(),
            "Unknown format: 'custom'. Adjust configuration to ignore unrecognized formats"
        );
    }

    #[test_case("2023-01-01", true; "valid regular date")]
    #[test_case("2020-02-29", true; "valid leap year date")]
    #[test_case("2021-02-28", true; "valid non-leap year date")]
    #[test_case("1900-02-28", true; "valid century non-leap year")]
    #[test_case("2000-02-29", true; "valid leap century year")]
    #[test_case("1999-12-31", true; "valid end of year date")]
    #[test_case("202-12-01", false; "invalid short year")]
    #[test_case("2023-1-01", false; "invalid short month")]
    #[test_case("2023-12-1", false; "invalid short day")]
    #[test_case("2023/12/01", false; "invalid separators")]
    #[test_case("2023-13-01", false; "invalid month too high")]
    #[test_case("2023-00-01", false; "invalid month too low")]
    #[test_case("2023-12-32", false; "invalid day too high")]
    #[test_case("2023-11-31", false; "invalid day for 30-day month")]
    #[test_case("2023-02-30", false; "invalid day for February in non-leap year")]
    #[test_case("2021-02-29", false; "invalid day for non-leap year")]
    #[test_case("2023-12-00", false; "invalid day too low")]
    #[test_case("99999-12-01", false; "year too long")]
    #[test_case("1900-02-29", false; "invalid leap century non-leap year")]
    #[test_case("2000-02-30", false; "invalid day for leap century year")]
    #[test_case("2400-02-29", true; "valid leap year in distant future")]
    #[test_case("0000-01-01", true; "valid boundary start date")]
    #[test_case("9999-12-31", true; "valid boundary end date")]
    #[test_case("aaaa-01-12", false; "Malformed (letters in year)")]
    #[test_case("2000-bb-12", false; "Malformed (letters in month)")]
    #[test_case("2000-01-cc", false; "Malformed (letters in day)")]
    #[test_case("20:3-01-15", false; "colon in year")]
    #[test_case("20;3-01-15", false; "semicolon in year")]
    fn test_is_valid_date(input: &str, expected: bool) {
        assert_eq!(is_valid_date(input), expected);
    }

    #[test_case("23:59:59Z", true; "valid time with Z")]
    #[test_case("00:00:00Z", true; "valid midnight time with Z")]
    #[test_case("12:30:45.123Z", true; "valid time with fractional seconds and Z")]
    #[test_case("23:59:60Z", true; "valid leap second UTC time")]
    #[test_case("12:30:45+01:00", true; "valid time with positive offset")]
    #[test_case("12:30:45-01:00", true; "valid time with negative offset")]
    #[test_case("23:59:60+00:00", true; "valid leap second with offset UTC 00:00")]
    #[test_case("23:59:59+01:00", true; "valid time with +01:00 offset")]
    #[test_case("23:59:59A", false; "invalid time with non-Z/non-offset letter")]
    #[test_case("12:3:45Z", false; "invalid time with missing digit in minute")]
    #[test_case("12:30:4Z", false; "invalid time with missing digit in second")]
    #[test_case("12-30-45Z", false; "invalid time with wrong separator")]
    #[test_case("12:30:45Z+01:00", false; "invalid time with Z and offset together")]
    #[test_case("12:30:45A01:00", false; "invalid time with wrong separator between time and offset")]
    #[test_case("12:30:45++01:00", false; "invalid double plus in offset")]
    #[test_case("12:30:45+01:60", false; "invalid minute in offset")]
    #[test_case("12:30:45+24:00", false; "invalid hour in offset")]
    #[test_case("12:30:45.", false; "invalid time with incomplete fractional second")]
    #[test_case("24:00:00Z", false; "invalid hour > 23")]
    #[test_case("12:60:00Z", false; "invalid minute > 59")]
    #[test_case("12:30:61Z", false; "invalid second > 60")]
    #[test_case("12:30:60+01:00", false; "invalid leap second with non-UTC offset")]
    #[test_case("23:59:60Z+01:00", false; "invalid leap second with non-zero offset")]
    #[test_case("23:59:60+00:30", false; "invalid leap second with non-zero minute offset")]
    #[test_case("23:59:60Z", true; "valid leap second at the end of day")]
    #[test_case("23:59:60+00:00", true; "valid leap second with zero offset")]
    #[test_case("ab:59:59Z", false; "invalid time with letters in hour")]
    #[test_case("23:ab:59Z", false; "invalid time with letters in minute")]
    #[test_case("23:59:abZ", false; "invalid time with letters in second")]
    #[test_case("23:59:59aZ", false; "invalid time with letter after seconds")]
    #[test_case("12:30:45+ab:00", false; "invalid offset hour with letters")]
    #[test_case("12:30:45+01:ab", false; "invalid offset minute with letters")]
    #[test_case("12:30:45.abcZ", false; "invalid fractional seconds with letters")]
    fn test_is_valid_time(input: &str, expected: bool) {
        assert_eq!(is_valid_time(input), expected);
    }

    #[test]
    fn test_is_valid_datetime() {
        assert!(!is_valid_datetime(""));
    }

    #[test_case("127.0.0.1", true)]
    #[test_case("192.168.1.1", true)]
    #[test_case("10.0.0.1", true)]
    #[test_case("0.0.0.0", true)]
    #[test_case("256.1.2.3", false; "first octet too large")]
    #[test_case("1.256.3.4", false; "second octet too large")]
    #[test_case("1.2.256.4", false; "third octet too large")]
    #[test_case("1.2.3.256", false; "fourth octet too large")]
    #[test_case("01.2.3.4", false; "leading zero in first octet")]
    #[test_case("1.02.3.4", false; "leading zero in second octet")]
    #[test_case("1.2.03.4", false; "leading zero in third octet")]
    #[test_case("1.2.3.04", false; "leading zero in fourth octet")]
    #[test_case("1.2.3", false; "too few octets")]
    #[test_case("1.2.3.4.5", false; "too many octets")]
    fn ip_v4(input: &str, expected: bool) {
        let validator = crate::options()
            .should_validate_formats(true)
            .build(&json!({"format": "ipv4", "type": "string"}))
            .expect("Invalid schema");
        assert_eq!(validator.is_valid(&json!(input)), expected);
    }

    #[test]
    fn test_is_valid_datetime_panic() {
        let _ = is_valid_datetime("2624-04-25t23:14:04-256\x112");
    }

    #[cfg(feature = "idna")]
    #[test_case("example.com" ; "simple valid hostname")]
    #[test_case("xn--bcher-kva.com" ; "valid punycode")]
    #[test_case("münchen.de" ; "valid IDN")]
    #[test_case("test\u{094D}\u{200D}example.com" ; "valid zero width joiner after virama")]
    #[test_case("۱۲۳.example.com" ; "valid extended arabic-indic digits")]
    #[test_case("ひらがな・カタカナ.com" ; "valid katakana middle dot")]
    fn test_valid_idn_hostnames(input: &str) {
        assert!(is_valid_idn_hostname(input));
    }

    #[test_case("xn--ll-0ea" ; "punycode with valid middle dot context")]
    #[test_case("xn--11b2ezcw70k" ; "zero width joiner preceded by virama")]
    fn test_valid_punycode_hostnames(input: &str) {
        assert!(is_valid_hostname(input));
    }

    #[cfg(feature = "idna")]
    #[test_case("ex--ample.com" ; "hyphen at 3rd & 4th position")]
    #[test_case("-example.com" ; "leading hyphen")]
    #[test_case("example-.com" ; "trailing hyphen")]
    #[test_case("xn--example.com" ; "invalid punycode")]
    #[test_case("xn--x" ; "too short punycode label")]
    #[test_case("xn--vek" ; "katakana middle dot without companions")]
    #[test_case("xn--l-fda" ; "middle dot with nothing preceding")]
    #[test_case("xn--l-gda" ; "middle dot with nothing following")]
    #[test_case("xn--02b508i" ; "zero width joiner not preceded by virama")]
    #[test_case("xn--a-2hc5h" ; "hebrew geresh not preceded by hebrew")]
    #[test_case("xn--a-2hc8h" ; "hebrew gershayim not preceded by hebrew")]
    #[test_case("test\u{200D}example.com" ; "zero width joiner not after virama")]
    #[test_case("test\u{0061}\u{200D}example.com" ; "zero width joiner after non-virama")]
    #[test_case("" ; "empty string")]
    #[test_case("." ; "single dot")]
    #[test_case("example..com" ; "consecutive dots")]
    #[test_case("exa mple.com" ; "contains space")]
    #[test_case("example.com." ; "trailing dot")]
    #[test_case("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.com" ; "too long")]
    #[test_case("xn--bcher-.com" ; "invalid punycode with hyphen")]
    #[test_case("١۲٣.example.com" ; "mixed arabic-indic digits")]
    #[test_case("example・com" ; "katakana middle dot without hiragana/katakana/han")]
    fn test_invalid_idn_hostnames(input: &str) {
        assert!(!is_valid_idn_hostname(input));
    }

    #[test_case("xn--l-fda" ; "middle dot with nothing preceding")]
    #[test_case("xn--l-gda" ; "middle dot with nothing following")]
    #[test_case("xn--02b508i" ; "zero width joiner not preceded by anything")]
    #[test_case("xn--11b2er09f" ; "zero width joiner not preceded by virama")]
    #[test_case("xn--hello-zed" ; "punycode beginning with nonspacing mark")]
    #[test_case("xn--hello-txk" ; "punycode beginning with spacing combining mark")]
    #[test_case("xn--hello-6bf" ; "punycode beginning with enclosing mark")]
    #[test_case("XN--aa---o47jg78q" ; "uppercase punycode prefix rejected")]
    fn test_invalid_punycode_hostnames(input: &str) {
        assert!(!is_valid_hostname(input));
    }

    #[test_case(Draft::Draft4 ; "draft4")]
    #[test_case(Draft::Draft6 ; "draft6")]
    fn test_hostname_a_label_rules_not_applied_in_legacy_drafts(draft: Draft) {
        let schema = json!({"format": "hostname", "type": "string"});
        let validator = crate::options()
            .with_draft(draft)
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        assert!(validator.is_valid(&json!("XN--9krT00a")));
        assert!(validator.is_valid(&json!("ex--ample.com")));
    }

    #[test_case(Draft::Draft7 ; "draft7")]
    #[test_case(Draft::Draft201909 ; "draft2019-09")]
    #[test_case(Draft::Draft202012 ; "draft2020-12")]
    fn test_hostname_a_label_rules_applied_in_modern_drafts(draft: Draft) {
        let schema = json!({"format": "hostname", "type": "string"});
        let validator = crate::options()
            .with_draft(draft)
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        assert!(!validator.is_valid(&json!("XN--9krT00a")));
        assert!(!validator.is_valid(&json!("ex--ample.com")));
    }

    #[test]
    fn test_invalid_hostname() {
        assert!(!is_valid_hostname("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.com"));
    }

    #[test_case(""; "empty string")]
    #[test_case("/"; "root")]
    #[test_case("/foo"; "simple key")]
    #[test_case("/foo/0"; "array index")]
    #[test_case("/foo/bar"; "nested keys")]
    #[test_case("/f~0o/b~1r"; "escaped characters")]
    #[test_case("/foo/bar/"; "trailing slash")]
    #[test_case("/foo//bar"; "empty reference token")]
    fn test_valid_json_pointer(pointer: &str) {
        assert!(is_valid_json_pointer(pointer));
    }

    #[test_case("foo"; "missing leading slash")]
    #[test_case("/foo/~"; "incomplete escape")]
    #[test_case("/foo/~2"; "invalid escape")]
    #[test_case("/foo\x7E"; "unescaped tilde")]
    fn test_invalid_json_pointer(pointer: &str) {
        assert!(!is_valid_json_pointer(pointer));
    }

    #[test_case("0"; "zero")]
    #[test_case("1"; "positive integer")]
    #[test_case("10"; "multi-digit integer")]
    #[test_case("0#"; "zero with hash")]
    #[test_case("1#"; "positive integer with hash")]
    #[test_case("0/"; "zero with slash")]
    #[test_case("1/foo"; "integer with json pointer")]
    #[test_case("10/foo/bar"; "multi-digit integer with json pointer")]
    fn test_valid_relative_json_pointer(pointer: &str) {
        assert!(is_valid_relative_json_pointer(pointer));
    }

    #[test_case(""; "empty string")]
    #[test_case("-1"; "negative integer")]
    #[test_case("01"; "leading zero")]
    #[test_case("1.5"; "decimal")]
    #[test_case("a"; "non-digit")]
    #[test_case("1a"; "digit followed by non-digit")]
    #[test_case("1#/"; "hash not at end")]
    #[test_case("1/~"; "incomplete escape in json pointer")]
    fn test_invalid_relative_json_pointer(pointer: &str) {
        assert!(!is_valid_relative_json_pointer(pointer));
    }

    #[test]
    fn email_options_backward_compatibility() {
        // Test that default behavior is unchanged (backward compatibility)
        let schema = json!({"format": "email", "type": "string"});
        let validator = crate::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        // "missing@domain" should validate as valid with default options (per spec)
        assert!(validator.is_valid(&json!("missing@domain")));
        assert!(validator.is_valid(&json!("user@example.com")));
        assert!(!validator.is_valid(&json!("not-an-email")));
    }

    #[test]
    fn email_options_custom() {
        let schema = json!({"format": "email", "type": "string"});

        // Test with custom email options
        let validator = crate::options()
            .with_email_options(EmailOptions::default())
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        // Should still work with custom options (same as default for now)
        assert!(validator.is_valid(&json!("user@example.com")));
        assert!(!validator.is_valid(&json!("not-an-email")));
    }

    #[test]
    fn email_options_default() {
        let schema = json!({"format": "email", "type": "string"});
        let validator = crate::options()
            .with_email_options(EmailOptions::default())
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        assert!(validator.is_valid(&json!("user@example.com")));
        assert!(!validator.is_valid(&json!("not-an-email")));
    }

    #[cfg(feature = "idna")]
    #[test]
    fn idn_email_options() {
        let schema = json!({"format": "idn-email", "type": "string"});
        let validator = crate::options()
            .with_email_options(EmailOptions::default())
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        assert!(validator.is_valid(&json!("user@example.com")));
        assert!(!validator.is_valid(&json!("not-an-email")));
    }

    #[cfg(feature = "idna")]
    #[test_case("δοκιμή@example.com", true; "non-ASCII local part")]
    #[test_case("\"δοκιμή\"@example.com", true; "non-ASCII quoted local part")]
    #[test_case("user@example.com", true; "ascii local part")]
    #[test_case("not-an-email", false; "no domain")]
    #[test_case("δοκιμή@-bad-.com", false; "non-ASCII local part with invalid domain")]
    fn idn_email_non_ascii(input: &str, expected: bool) {
        let validator = crate::options()
            .should_validate_formats(true)
            .build(&json!({"format": "idn-email", "type": "string"}))
            .expect("Schema should compile");
        assert_eq!(validator.is_valid(&json!(input)), expected);
    }

    // `email` keeps `email_address` parsing: it rejects a non-ASCII quoted local part,
    // unlike `idn-email` which masks it through. (An unquoted non-ASCII local part is
    // accepted by the crate for both.)
    #[test_case("\"δοκιμή\"@example.com", false; "non-ASCII quoted local part")]
    #[test_case("user@example.com", true; "ascii local part")]
    fn email_ascii_only_local_part(input: &str, expected: bool) {
        let validator = crate::options()
            .should_validate_formats(true)
            .build(&json!({"format": "email", "type": "string"}))
            .expect("Schema should compile");
        assert_eq!(validator.is_valid(&json!(input)), expected);
    }

    #[test]
    fn email_options_minimum_sub_domains() {
        let schema = json!({"format": "email", "type": "string"});

        // Test with no minimum sub domains - localhost should be valid
        let validator = crate::options()
            .with_email_options(EmailOptions::default().with_no_minimum_sub_domains())
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        assert!(validator.is_valid(&json!("simon@localhost")));
        assert!(validator.is_valid(&json!("user@example.com")));

        // Test with required TLD - localhost should be invalid
        let validator = crate::options()
            .with_email_options(EmailOptions::default().with_required_tld())
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        assert!(!validator.is_valid(&json!("simon@localhost")));
        assert!(validator.is_valid(&json!("user@example.com")));

        // Test with custom minimum sub domains
        let validator = crate::options()
            .with_email_options(EmailOptions::default().with_minimum_sub_domains(3))
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        assert!(!validator.is_valid(&json!("user@example.com")));
        assert!(validator.is_valid(&json!("user@sub.example.com")));
    }

    #[test]
    fn email_options_domain_literal() {
        let schema = json!({"format": "email", "type": "string"});

        // Test with domain literal allowed (default)
        let validator = crate::options()
            .with_email_options(EmailOptions::default().with_domain_literal())
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        // Domain literal format is allowed (even if IPv4 is invalid)
        assert!(validator.is_valid(&json!("email@[127.0.0.1]")));
        assert!(validator.is_valid(&json!("email@[IPv6:2001:db8::1]")));

        // Test without domain literal - should reject domain literals
        let validator = crate::options()
            .with_email_options(EmailOptions::default().without_domain_literal())
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        assert!(!validator.is_valid(&json!("email@[127.0.0.1]")));
        assert!(!validator.is_valid(&json!("email@[IPv6:2001:db8::1]")));
        assert!(validator.is_valid(&json!("user@example.com")));
    }

    #[test]
    fn email_options_display_text() {
        let schema = json!({"format": "email", "type": "string"});

        // Test with display text allowed (default)
        let validator = crate::options()
            .with_email_options(EmailOptions::default().with_display_text())
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        // Display text format with actual display name should be allowed
        assert!(validator.is_valid(&json!("Simon <simon@example.com>")));
        // Plain email should always be valid
        assert!(validator.is_valid(&json!("simon@example.com")));

        // Test without display text - should reject display text formats
        let validator = crate::options()
            .with_email_options(EmailOptions::default().without_display_text())
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        assert!(!validator.is_valid(&json!("Simon <simon@example.com>")));
        assert!(!validator.is_valid(&json!("<simon@example.com>")));
        assert!(validator.is_valid(&json!("simon@example.com")));
    }

    #[test]
    fn email_options_combined() {
        let schema = json!({"format": "email", "type": "string"});

        // Test combining multiple options - strict validation
        let validator = crate::options()
            .with_email_options(
                EmailOptions::default()
                    .with_required_tld()
                    .without_domain_literal()
                    .without_display_text(),
            )
            .should_validate_formats(true)
            .build(&schema)
            .expect("Schema should compile");

        // Should reject addresses without TLD
        assert!(!validator.is_valid(&json!("user@localhost")));
        // Should reject domain literals
        assert!(!validator.is_valid(&json!("user@[127.0.0.1]")));
        // Should reject display text
        assert!(!validator.is_valid(&json!("Name <user@example.com>")));
        // Should accept valid email with TLD
        assert!(validator.is_valid(&json!("user@example.com")));
    }

    // Simple valid templates
    #[test_case(""; "empty string")]
    #[test_case("http://example.com/"; "plain URL")]
    #[test_case("no-template-here"; "no templates")]
    #[test_case("{var}"; "simple variable")]
    #[test_case("http://example.com/{var}"; "URL with variable")]
    #[test_case("http://example.com/dictionary/{term}"; "URL with named variable")]
    #[test_case("/users/{id}"; "path with variable")]
    #[test_case("http://example.com/~{username}/"; "URL with tilde")]
    // All operators
    #[test_case("{+var}"; "reserved expansion")]
    #[test_case("{#var}"; "fragment expansion")]
    #[test_case("{.var}"; "label expansion with dot")]
    #[test_case("{/var}"; "path segment expansion")]
    #[test_case("{;var}"; "path-style parameter expansion")]
    #[test_case("{?var}"; "query expansion")]
    #[test_case("{&var}"; "query continuation expansion")]
    // RFC 6570 reserved operators (Section 2.2, Appendix A) - syntactically valid
    #[test_case("{=var}"; "reserved operator equals")]
    #[test_case("{,var}"; "reserved operator comma")]
    #[test_case("{!var}"; "reserved operator exclamation")]
    #[test_case("{@var}"; "reserved operator at")]
    #[test_case("{|var}"; "reserved operator pipe")]
    #[test_case("{|var*}"; "reserved operator with explode")]
    // Prefix modifier valid on any variable (RFC 6570 Section 2.4.1)
    #[test_case("{keys:1}"; "prefix modifier on any var")]
    #[test_case("{+keys:1}"; "operator with prefix modifier")]
    // Modifiers
    #[test_case("{var:10}"; "prefix modifier")]
    #[test_case("{var:1}"; "prefix modifier min")]
    #[test_case("{var:9999}"; "prefix modifier max")]
    #[test_case("{var*}"; "explode modifier")]
    #[test_case("{+var*}"; "operator with explode")]
    #[test_case("{#var:5}"; "operator with prefix")]
    // Multiple variables
    #[test_case("{var1,var2}"; "multiple variables")]
    #[test_case("{var1,var2,var3}"; "three variables")]
    #[test_case("{+var1,var2}"; "operator with multiple variables")]
    #[test_case("{var1:5,var2*}"; "multiple variables with modifiers")]
    // Complex templates
    #[test_case("http://example.com{+path}{?query*}"; "complex template")]
    #[test_case("http://example.com{#fragment}"; "fragment template")]
    #[test_case("http://example.com{.dom*}"; "domain template")]
    #[test_case("http://example.com{/path,path2}"; "path template")]
    #[test_case("http://example.com{;params*}"; "params template")]
    #[test_case("http://example.com{?query,query2}"; "query template")]
    #[test_case("http://example.com/{var1}/{var2}/{var3}"; "multiple expressions")]
    // Variable names with dots
    #[test_case("{var.name}"; "dotted variable name")]
    #[test_case("{a.b.c}"; "multiple dots in variable name")]
    // Percent-encoded in variable names
    #[test_case("{%20}"; "percent-encoded space in varname")]
    #[test_case("{a%20b}"; "percent-encoded in middle of varname")]
    #[test_case("{%41}"; "percent-encoded A")]
    // Percent-encoded in literals
    #[test_case("http://example.com/%20space"; "percent-encoded in URL")]
    #[test_case("hello%20world"; "percent-encoded space")]
    // RFC 6570 errata 6937: `'{var}'` from the RFC's own examples is a valid template
    #[test_case("a'b"; "apostrophe in literal")]
    fn test_valid_uri_template(template: &str) {
        assert!(is_valid_uri_template(template));
    }

    // Invalid templates
    #[test_case("{"; "unclosed brace")]
    #[test_case("}"; "unmatched close brace")]
    #[test_case("{}"; "empty expression")]
    #[test_case("{+}"; "operator only")]
    #[test_case("http://example.com/{unclosed"; "unclosed in URL")]
    #[test_case("http://example.com/{var"; "missing close brace")]
    #[test_case("http://example.com/}"; "extra close brace")]
    // Invalid modifiers
    #[test_case("{var:0}"; "prefix zero")]
    #[test_case("{var:}"; "prefix empty")]
    #[test_case("{var:10000}"; "prefix too large")]
    #[test_case("{var::5}"; "double colon")]
    #[test_case("{var**}"; "double explode")]
    #[test_case("{*}"; "explode only")]
    #[test_case("{:5}"; "prefix only")]
    // Invalid variable names
    #[test_case("{-var}"; "hyphen start")]
    #[test_case("{var-}"; "hyphen in variable")]
    #[test_case("{.}"; "dot only")]
    #[test_case("{..var}"; "double dot start")]
    #[test_case("{var..name}"; "double dot in name")]
    // Invalid percent encoding
    #[test_case("{%}"; "incomplete percent")]
    #[test_case("{%Z}"; "incomplete percent hex")]
    #[test_case("{%ZZ}"; "invalid hex digits")]
    #[test_case("{%0}"; "single hex digit")]
    #[test_case("%"; "incomplete percent in literal")]
    #[test_case("%Z"; "incomplete percent in literal 2")]
    #[test_case("%ZZ"; "invalid hex in literal")]
    // Invalid characters in literals
    #[test_case("hello world"; "space in literal")]
    #[test_case("hello\ttab"; "tab in literal")]
    #[test_case("hello\nline"; "newline in literal")]
    #[test_case("hello\"quote"; "quote in literal")]
    #[test_case("hello<angle"; "angle bracket in literal")]
    #[test_case("hello\\back"; "backslash in literal")]
    #[test_case("hello^caret"; "caret in literal")]
    #[test_case("hello`backtick"; "backtick in literal")]
    #[test_case("hello|pipe"; "pipe in literal")]
    // Invalid expressions
    #[test_case("{var,}"; "trailing comma")]
    #[test_case("{var,,var2}"; "double comma")]
    // uritemplate-test/negative-tests.json
    // Note: Per RFC 6570, reserved operators (=, ,, !, @, |) are syntactically valid.
    // Tests 7, 11, 13, 21, 22 are valid per RFC syntax but fail in uritemplate-test
    // because they test expansion behavior, not syntax validation.
    #[test_case("{/id*"; "uritemplate-test 1: unclosed brace")]
    #[test_case("/id*}"; "uritemplate-test 2: unmatched close brace")]
    #[test_case("{/?id}"; "uritemplate-test 3: question mark in varname")]
    #[test_case("{var:prefix}"; "uritemplate-test 4: non-numeric prefix")]
    #[test_case("{hello:2*}"; "uritemplate-test 5: prefix and explode combined")]
    #[test_case("{??hello}"; "uritemplate-test 6: question mark in varname")]
    #[test_case("{with space}"; "uritemplate-test 8: space in variable name")]
    #[test_case("{ leading_space}"; "uritemplate-test 9: leading space in expression")]
    #[test_case("{trailing_space }"; "uritemplate-test 10: trailing space in expression")]
    #[test_case("{$var}"; "uritemplate-test 12: dollar sign not a valid operator")]
    #[test_case("{*keys?}"; "uritemplate-test 14: explode at start of varspec")]
    #[test_case("{?empty=default,var}"; "uritemplate-test 15: equals sign in varname")]
    #[test_case("{var}{-prefix|/-/|var}"; "uritemplate-test 16: hyphen not a valid operator")]
    #[test_case("?q={searchTerms}&amp;c={example:color?}"; "uritemplate-test 17: non-numeric prefix")]
    #[test_case("x{?empty|foo=none}"; "uritemplate-test 18: invalid chars after varname")]
    #[test_case("/h{#hello+}"; "uritemplate-test 19: plus in varname")]
    #[test_case("/h#{hello+}"; "uritemplate-test 20: plus in varname")]
    #[test_case("{;keys:1*}"; "uritemplate-test 23: prefix and explode combined")]
    #[test_case("?{-join|&|var,list}"; "uritemplate-test 24: hyphen not a valid operator")]
    #[test_case("{~thing}"; "uritemplate-test 25: tilde not a valid operator")]
    #[test_case("/{default-graph-uri}"; "uritemplate-test 26: hyphen in varname")]
    #[test_case("/sparql{?query,default-graph-uri}"; "uritemplate-test 27: hyphen in varname")]
    #[test_case("/sparql{?query){&default-graph-uri*}"; "uritemplate-test 28: paren in template")]
    #[test_case("/resolution{?x, y}"; "uritemplate-test 29: space after comma")]
    fn test_invalid_uri_template(template: &str) {
        assert!(!is_valid_uri_template(template));
    }

    #[cfg(feature = "macros")]
    #[test]
    fn is_valid_regex_cache_matches_fresh_parse() {
        let mut patterns: Vec<String> = (0..20).map(|i| format!("^val{i}[a-z]+$")).collect();
        patterns.extend((0..20).map(|i| format!("[unclosed{i}")));
        for _ in 0..3 {
            for pattern in &patterns {
                assert_eq!(
                    is_valid_regex(pattern),
                    jsonschema_regex::to_rust_regex(pattern).is_ok(),
                    "pattern {pattern}"
                );
            }
        }
    }

    #[test]
    fn every_builtin_format_example_satisfies_its_own_format() {
        for format in BuiltinFormat::VARIANTS {
            assert!(
                format.is_valid(format.example()),
                "{format:?} example `{}` does not satisfy `{}`",
                format.example(),
                format.as_str()
            );
            if let Some((minimum, maximum)) = format.length_window() {
                let length = format.example().chars().count() as u64;
                assert!(
                    (minimum..=maximum).contains(&length),
                    "{format:?} example `{}` falls outside its declared length window",
                    format.example()
                );
            }
        }
    }

    /// The hostname ceiling, too long to write out as a case above.
    #[test]
    fn the_longest_hostname_is_accepted() {
        let label = "a".repeat(63);
        let hostname = format!("{label}.{label}.{label}.{}", "a".repeat(61));
        assert_eq!(hostname.len(), 253);
        assert!(is_valid_hostname(&hostname));
        assert_eq!(BuiltinFormat::Hostname.length_window(), Some((1, 253)));
    }

    /// The exact window ends, which no corpus is guaranteed to carry.
    #[test_case("00:00:00Z", BuiltinFormat::Time; "shortest time")]
    #[test_case("0000-01-01T00:00:00Z", BuiltinFormat::DateTime; "shortest date-time")]
    #[test_case("P1D", BuiltinFormat::Duration; "shortest duration")]
    #[test_case("::", BuiltinFormat::Ipv6; "shortest ipv6")]
    #[test_case(
        "0000:0000:0000:0000:0000:0000:255.255.255.255",
        BuiltinFormat::Ipv6;
        "longest ipv6"
    )]
    #[test_case("a", BuiltinFormat::Hostname; "shortest hostname")]
    fn test_window_end_is_accepted(text: &str, format: BuiltinFormat) {
        assert!(format.is_valid(text), "`{text}` is not a valid {format:?}");
        let (minimum, maximum) = format.length_window().expect("has a window");
        let length = text.chars().count() as u64;
        assert!(
            length == minimum || length == maximum,
            "`{text}` sits inside the window rather than at an end"
        );
    }

    #[test]
    fn malformed() {
        tests_util::assert_compile_error(
            &json!({"format": 5}),
            "5 is not of type \"string\"",
            "/format",
        );
    }
}
