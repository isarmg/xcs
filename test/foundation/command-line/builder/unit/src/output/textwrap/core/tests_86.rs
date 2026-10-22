#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "unicode")]
    use unicode_width::UnicodeWidthChar;

    #[test]
    fn emojis_have_correct_width() {
        // The supported layout contract counts every scalar as one column.
        // Cover all scalars in these ranges, a strict superset of the original
        // emoji-only checks, without an unrelated emoji-table dependency.

        // Emojis in the Basic Latin (ASCII) and Latin-1 Supplement
        // blocks all have a width of 1 column. This includes
        // characters such as '#' and '©'.
        for ch in '\u{1}'..'\u{FF}' {
            {
                let desc = format!("{:?} U+{:04X}", ch, ch as u32);

                #[cfg(feature = "unicode")]
                assert_eq!(ch.width().unwrap(), 1, "char: {desc}");

                #[cfg(not(feature = "unicode"))]
                assert_eq!(ch_width(ch), 1, "char: {desc}");
            }
        }

        // Emojis in the remaining blocks of the Basic Multilingual
        // Plane (BMP), in the Supplementary Multilingual Plane (SMP),
        // and in the Supplementary Ideographic Plane (SIP), are all 1
        // or 2 columns wide when unicode-width is used, and always 1
        // column wide otherwise. This includes all of our favorite
        // emojis such as 😊.
        for ch in '\u{FF}'..'\u{2FFFF}' {
            {
                let desc = format!("{:?} U+{:04X}", ch, ch as u32);

                #[cfg(feature = "unicode")]
                assert!(ch.width().unwrap() <= 2, "char: {desc}");

                #[cfg(not(feature = "unicode"))]
                assert_eq!(ch_width(ch), 1, "char: {desc}");
            }
        }

        // The remaining planes contain almost no assigned code points
        // and thus also no emojis.
    }

    #[test]
    #[cfg(feature = "unicode")]
    fn display_width_works() {
        assert_eq!("Café Plain".len(), 11); // “é” is two bytes
        assert_eq!(display_width("Café Plain"), 10);
    }

    #[test]
    #[cfg(feature = "unicode")]
    fn display_width_narrow_emojis() {
        assert_eq!(display_width("⁉"), 1);
    }

    #[test]
    #[cfg(feature = "unicode")]
    fn display_width_narrow_emojis_variant_selector() {
        assert_eq!(display_width("⁉\u{fe0f}"), 1);
    }

    #[test]
    #[cfg(feature = "unicode")]
    fn display_width_emojis() {
        assert_eq!(display_width("😂😭🥺🤣✨😍🙏🥰😊🔥"), 20);
    }
}
