#[cfg(test)]
mod tests {
    pub use super::super::{AnsiGenericString, AnsiStrings};
    pub use crate::style::Color::*;
    pub use crate::style::Style;

    #[test]
    fn no_control_codes_for_plain() {
        let one = Style::default().paint("one");
        let two = Style::default().paint("two");
        let output = AnsiStrings(&[one, two]).to_string();
        assert_eq!(output, "onetwo");
    }

    // NOTE: unstyled because it could have OSC escape sequences
    fn idempotent(unstyled: AnsiGenericString<'_, str>) {
        let before_g = Green.paint("Before is Green. ");
        let before = Style::default().paint("Before is Plain. ");
        let after_g = Green.paint(" After is Green.");
        let after = Style::default().paint(" After is Plain.");
        let unstyled_s = unstyled.clone().to_string();

        // check that RESET precedes unstyled
        let joined = AnsiStrings(&[before_g.clone(), unstyled.clone()]).to_string();
        assert!(joined.starts_with("\x1B[32mBefore is Green. \x1B[0m"));
        assert!(
            joined.ends_with(unstyled_s.as_str()),
            "{:?} does not end with {:?}",
            joined,
            unstyled_s
        );

        // check that RESET does not follow unstyled when appending styled
        let joined = AnsiStrings(&[unstyled.clone(), after_g.clone()]).to_string();
        assert!(
            joined.starts_with(unstyled_s.as_str()),
            "{:?} does not start with {:?}",
            joined,
            unstyled_s
        );
        assert!(joined.ends_with("\x1B[32m After is Green.\x1B[0m"));

        // does not introduce spurious SGR codes (reset or otherwise) adjacent
        // to plain strings
        let joined = AnsiStrings(&[unstyled.clone()]).to_string();
        assert!(
            !joined.contains("\x1B["),
            "{:?} does contain \\x1B[",
            joined
        );
        let joined = AnsiStrings(&[before.clone(), unstyled.clone()]).to_string();
        assert!(
            !joined.contains("\x1B["),
            "{:?} does contain \\x1B[",
            joined
        );
        let joined = AnsiStrings(&[before.clone(), unstyled.clone(), after.clone()]).to_string();
        assert!(
            !joined.contains("\x1B["),
            "{:?} does contain \\x1B[",
            joined
        );
        let joined = AnsiStrings(&[unstyled.clone(), after.clone()]).to_string();
        assert!(
            !joined.contains("\x1B["),
            "{:?} does contain \\x1B[",
            joined
        );
    }

    #[test]
    fn title() {
        let title = AnsiGenericString::title("Test Title");
        assert_eq!(title.clone().to_string(), "\x1B]2;Test Title\x1B\\");
        idempotent(title)
    }

    #[test]
    fn hyperlink() {
        let styled = Red
            .paint("Link to example.com.")
            .hyperlink("https://example.com");
        assert_eq!(
            styled.to_string(),
            "\x1B[31m\x1B]8;;https://example.com\x1B\\Link to example.com.\x1B]8;;\x1B\\\x1B[0m"
        );
    }

    #[test]
    fn hyperlinks() {
        let before = Green.paint("Before link. ");
        let link = Blue
            .underline()
            .paint("Link to example.com.")
            .hyperlink("https://example.com");
        let after = Green.paint(" After link.");

        // Assemble with link by itself
        let joined = AnsiStrings(&[link.clone()]).to_string();
        #[cfg(feature = "gnu_legacy")]
        assert_eq!(joined, format!("\x1B[04;34m\x1B]8;;https://example.com\x1B\\Link to example.com.\x1B]8;;\x1B\\\x1B[0m"));
        #[cfg(not(feature = "gnu_legacy"))]
        assert_eq!(joined, format!("\x1B[4;34m\x1B]8;;https://example.com\x1B\\Link to example.com.\x1B]8;;\x1B\\\x1B[0m"));

        // Assemble with link in the middle
        let joined = AnsiStrings(&[before.clone(), link.clone(), after.clone()]).to_string();
        #[cfg(feature = "gnu_legacy")]
        assert_eq!(joined, format!("\x1B[32mBefore link. \x1B[04;34m\x1B]8;;https://example.com\x1B\\Link to example.com.\x1B]8;;\x1B\\\x1B[0m\x1B[32m After link.\x1B[0m"));
        #[cfg(not(feature = "gnu_legacy"))]
        assert_eq!(joined, format!("\x1B[32mBefore link. \x1B[4;34m\x1B]8;;https://example.com\x1B\\Link to example.com.\x1B]8;;\x1B\\\x1B[0m\x1B[32m After link.\x1B[0m"));

        // Assemble with link first
        let joined = AnsiStrings(&[link.clone(), after.clone()]).to_string();
        #[cfg(feature = "gnu_legacy")]
        assert_eq!(joined, format!("\x1B[04;34m\x1B]8;;https://example.com\x1B\\Link to example.com.\x1B]8;;\x1B\\\x1B[0m\x1B[32m After link.\x1B[0m"));
        #[cfg(not(feature = "gnu_legacy"))]
        assert_eq!(joined, format!("\x1B[4;34m\x1B]8;;https://example.com\x1B\\Link to example.com.\x1B]8;;\x1B\\\x1B[0m\x1B[32m After link.\x1B[0m"));

        // Assemble with link at the end
        let joined = AnsiStrings(&[before.clone(), link.clone()]).to_string();
        #[cfg(feature = "gnu_legacy")]
        assert_eq!(joined, format!("\x1B[32mBefore link. \x1B[04;34m\x1B]8;;https://example.com\x1B\\Link to example.com.\x1B]8;;\x1B\\\x1B[0m"));
        #[cfg(not(feature = "gnu_legacy"))]
        assert_eq!(joined, format!("\x1B[32mBefore link. \x1B[4;34m\x1B]8;;https://example.com\x1B\\Link to example.com.\x1B]8;;\x1B\\\x1B[0m"));
    }
}
