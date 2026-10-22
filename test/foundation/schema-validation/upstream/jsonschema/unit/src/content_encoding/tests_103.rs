#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    // Test string: "foobar"
    const TEST_STRING: &str = "foobar";
    const TEST_BASE64: &str = "Zm9vYmFy";
    const TEST_BASE64URL: &str = "Zm9vYmFy"; // same as base64 for "foobar" (no +/)
    const TEST_BASE32: &str = "MZXW6YTBOI======";
    const TEST_BASE32HEX: &str = "CPNMUOJ1E8======";
    const TEST_BASE16_UPPER: &str = "666F6F626172";
    const TEST_BASE16_LOWER: &str = "666f6f626172";
    const TEST_BASE16_MIXED: &str = "666F6f626172";

    #[test_case(TEST_BASE64, true ; "valid base64")]
    #[test_case("not valid base64!!!", false ; "invalid base64 with special chars")]
    #[test_case("Zm9v====", false ; "invalid base64 padding")]
    fn test_is_base64(input: &str, expected: bool) {
        assert_eq!(is_base64(input), expected);
    }

    #[test_case(TEST_BASE64, Some(TEST_STRING) ; "decode valid base64")]
    #[test_case("invalid!", None ; "decode invalid base64")]
    fn test_from_base64(input: &str, expected: Option<&str>) {
        assert_eq!(
            from_base64(input).unwrap(),
            expected.map(std::string::ToString::to_string)
        );
    }

    #[test_case(TEST_BASE64URL, true ; "valid base64url")]
    #[test_case("PDw_Pz4-", true ; "base64url with url safe chars")]
    #[test_case("Zm9v+YmFy", false ; "base64 plus char invalid in base64url")]
    #[test_case("Zm9v/YmFy", false ; "base64 slash char invalid in base64url")]
    fn test_is_base64url(input: &str, expected: bool) {
        assert_eq!(is_base64url(input), expected);
    }

    #[test_case(TEST_BASE64URL, Some(TEST_STRING) ; "decode valid base64url")]
    #[test_case("invalid!", None ; "decode invalid base64url")]
    fn test_from_base64url(input: &str, expected: Option<&str>) {
        assert_eq!(
            from_base64url(input).unwrap(),
            expected.map(std::string::ToString::to_string)
        );
    }

    #[test_case(TEST_BASE32, true ; "valid base32")]
    #[test_case("not valid", false ; "invalid base32 text")]
    #[test_case("189", false ; "base32 invalid chars 1,8,9")]
    fn test_is_base32(input: &str, expected: bool) {
        assert_eq!(is_base32(input), expected);
    }

    #[test_case(TEST_BASE32, Some(TEST_STRING) ; "decode valid base32")]
    #[test_case("189!!!", None ; "decode invalid base32")]
    fn test_from_base32(input: &str, expected: Option<&str>) {
        assert_eq!(
            from_base32(input).unwrap(),
            expected.map(std::string::ToString::to_string)
        );
    }

    #[test_case(TEST_BASE32HEX, true ; "valid base32hex")]
    #[test_case("not valid", false ; "invalid base32hex text")]
    #[test_case("XYZ", false ; "base32hex invalid chars X,Y,Z")]
    fn test_is_base32hex(input: &str, expected: bool) {
        assert_eq!(is_base32hex(input), expected);
    }

    #[test_case(TEST_BASE32HEX, Some(TEST_STRING) ; "decode valid base32hex")]
    #[test_case("XYZ!!!", None ; "decode invalid base32hex")]
    fn test_from_base32hex(input: &str, expected: Option<&str>) {
        assert_eq!(
            from_base32hex(input).unwrap(),
            expected.map(std::string::ToString::to_string)
        );
    }

    #[test_case(TEST_BASE16_UPPER, true ; "valid base16 uppercase")]
    #[test_case(TEST_BASE16_LOWER, true ; "valid base16 lowercase")]
    #[test_case(TEST_BASE16_MIXED, true ; "valid base16 mixed case")]
    #[test_case("not valid", false ; "invalid base16 text")]
    #[test_case("GHIJ", false ; "base16 invalid chars G-J")]
    fn test_is_base16(input: &str, expected: bool) {
        assert_eq!(is_base16(input), expected);
    }

    #[test_case(TEST_BASE16_UPPER, Some(TEST_STRING) ; "decode base16 uppercase")]
    #[test_case(TEST_BASE16_LOWER, Some(TEST_STRING) ; "decode base16 lowercase")]
    #[test_case(TEST_BASE16_MIXED, Some(TEST_STRING) ; "decode base16 mixed")]
    #[test_case("GHIJ", None ; "decode invalid base16")]
    fn test_from_base16(input: &str, expected: Option<&str>) {
        assert_eq!(
            from_base16(input).unwrap(),
            expected.map(std::string::ToString::to_string)
        );
    }
}
