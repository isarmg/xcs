#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    /// Regression test for <https://github.com/unicode-org/icu4x/issues/6041>
    fn test_scx_regression_6041() {
        let scripts = ScriptWithExtensions::new()
            .get_script_extensions_val('\u{2bc}')
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(
            scripts,
            [
                Script::Bengali,
                Script::Cyrillic,
                Script::Devanagari,
                Script::Latin,
                Script::Thai,
                Script::Lisu,
                Script::Toto
            ]
        );
    }

    #[test]
    fn test_high_discriminant() {
        let swe = ScriptWithExtensions::new();
        assert!(!swe.has_script32(0x0640, Script(0xAFFE)));
    }
}
