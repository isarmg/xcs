#[cfg(test)]
#[cfg(feature = "wrap_help")]
mod wrap_tests {
    use super::*;

    use snapbox::assert_data_eq;
    use snapbox::str;

    #[test]
    #[cfg(feature = "wrap_help")]
    fn wrap_unstyled() {
        let style = anstyle::Style::new();
        let input = format!(
            "{style}12345{style:#} {style}12345{style:#} {style}12345{style:#} {style}12345{style:#}"
        );
        let mut actual = StyledStr::new();
        actual.push_string(input);
        actual.wrap(20);
        assert_data_eq!(
            actual.ansi().to_string(),
            str![[r#"
12345 12345 12345
12345
"#]]
        );
    }

    #[test]
    #[cfg(feature = "wrap_help")]
    fn wrap_styled() {
        let style = anstyle::Style::new().bold();
        let input = format!(
            "{style}12345{style:#} {style}12345{style:#} {style}12345{style:#} {style}12345{style:#}"
        );
        let mut actual = StyledStr::new();
        actual.push_string(input);
        actual.wrap(20);
        assert_data_eq!(
            actual.ansi().to_string(),
            str![[r#"
[1m12345[0m [1m12345[0m [1m12345[0m [1m
12345[0m
"#]]
        );
    }
}
