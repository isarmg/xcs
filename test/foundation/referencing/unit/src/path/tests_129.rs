#[cfg(test)]
mod tests {
    use super::write_escaped_str;
    use test_case::test_case;

    #[test_case("", ""; "empty")]
    #[test_case("abc", "abc"; "short plain")]
    #[test_case("abcdefgh", "abcdefgh"; "one word plain")]
    #[test_case("abcdefghijklmnopq", "abcdefghijklmnopq"; "two words and a tail plain")]
    #[test_case("~", "~0"; "tilde alone")]
    #[test_case("/", "~1"; "slash alone")]
    #[test_case("a/b", "a~1b"; "slash inside")]
    #[test_case("abcdefg~", "abcdefg~0"; "tilde in the last byte of a word")]
    #[test_case("abcdefgh/", "abcdefgh~1"; "slash in the tail")]
    #[test_case("abcdefghijklmno~pqrstuvw", "abcdefghijklmno~0pqrstuvw"; "tilde in the second word")]
    #[test_case("~/~", "~0~1~0"; "every byte escaped")]
    #[test_case("a/b/c/d/e/f/g/h/i/j", "a~1b~1c~1d~1e~1f~1g~1h~1i~1j"; "escape in every word")]
    #[test_case("/v1/accounts/{account-key}", "~1v1~1accounts~1{account-key}"; "path key")]
    #[test_case("abcdefgh/ijklmnop~qrstuvwx", "abcdefgh~1ijklmnop~0qrstuvwx"; "escapes at word starts")]
    fn escapes(value: &str, expected: &str) {
        let mut buffer = String::from("/");
        write_escaped_str(&mut buffer, value);
        assert_eq!(buffer, format!("/{expected}"));
    }
}
