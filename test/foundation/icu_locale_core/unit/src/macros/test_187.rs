#[cfg(test)]
mod test {
    use crate::LanguageIdentifier;
    use crate::Locale;

    #[test]
    fn test_langid_macro_can_parse_langid_with_single_variant() {
        const DE_AT_FOOBAR: LanguageIdentifier = langid!("de-at-foobar");
        let de_at_foobar: LanguageIdentifier = "de-at-foobar".parse().unwrap();
        assert_eq!(DE_AT_FOOBAR, de_at_foobar);
    }

    #[test]
    fn test_locale_macro_can_parse_locale_with_single_variant() {
        const DE_AT_FOOBAR: Locale = locale!("de-at-foobar");
        let de_at_foobar: Locale = "de-at-foobar".parse().unwrap();
        assert_eq!(DE_AT_FOOBAR, de_at_foobar);
    }

    #[test]
    fn test_locale_macro_can_parse_locale_with_single_keyword_unicode_extension() {
        const DE_AT_U_CA_FOOBAR: Locale = locale!("de-at-u-ca-foobar");
        let de_at_u_ca_foobar: Locale = "de-at-u-ca-foobar".parse().unwrap();
        assert_eq!(DE_AT_U_CA_FOOBAR, de_at_u_ca_foobar);
    }
}
