#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! testlook {
        ($look:expr, $haystack:expr, $at:expr) => {
            LookMatcher::default().matches($look, $haystack.as_bytes(), $at)
        };
    }

    #[test]
    fn look_matches_start_line() {
        let look = Look::StartLF;

        assert!(testlook!(look, "", 0));
        assert!(testlook!(look, "\n", 0));
        assert!(testlook!(look, "\n", 1));
        assert!(testlook!(look, "a", 0));
        assert!(testlook!(look, "\na", 1));

        assert!(!testlook!(look, "a", 1));
        assert!(!testlook!(look, "a\na", 1));
    }

    #[test]
    fn look_matches_end_line() {
        let look = Look::EndLF;

        assert!(testlook!(look, "", 0));
        assert!(testlook!(look, "\n", 1));
        assert!(testlook!(look, "\na", 0));
        assert!(testlook!(look, "\na", 2));
        assert!(testlook!(look, "a\na", 1));

        assert!(!testlook!(look, "a", 0));
        assert!(!testlook!(look, "\na", 1));
        assert!(!testlook!(look, "a\na", 0));
        assert!(!testlook!(look, "a\na", 2));
    }

    #[test]
    fn look_matches_start_text() {
        let look = Look::Start;

        assert!(testlook!(look, "", 0));
        assert!(testlook!(look, "\n", 0));
        assert!(testlook!(look, "a", 0));

        assert!(!testlook!(look, "\n", 1));
        assert!(!testlook!(look, "\na", 1));
        assert!(!testlook!(look, "a", 1));
        assert!(!testlook!(look, "a\na", 1));
    }

    #[test]
    fn look_matches_end_text() {
        let look = Look::End;

        assert!(testlook!(look, "", 0));
        assert!(testlook!(look, "\n", 1));
        assert!(testlook!(look, "\na", 2));

        assert!(!testlook!(look, "\na", 0));
        assert!(!testlook!(look, "a\na", 1));
        assert!(!testlook!(look, "a", 0));
        assert!(!testlook!(look, "\na", 1));
        assert!(!testlook!(look, "a\na", 0));
        assert!(!testlook!(look, "a\na", 2));
    }

    #[test]
    #[cfg(all(not(miri), feature = "unicode-word-boundary"))]
    fn look_matches_word_unicode() {
        let look = Look::WordUnicode;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(testlook!(look, "a", 0));
        assert!(testlook!(look, "a", 1));
        assert!(testlook!(look, "a ", 1));
        assert!(testlook!(look, " a ", 1));
        assert!(testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint.
        assert!(testlook!(look, "𝛃", 0));
        assert!(testlook!(look, "𝛃", 4));
        assert!(testlook!(look, "𝛃 ", 4));
        assert!(testlook!(look, " 𝛃 ", 1));
        assert!(testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints.
        assert!(testlook!(look, "𝛃𐆀", 0));
        assert!(testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(!testlook!(look, "", 0));
        assert!(!testlook!(look, "ab", 1));
        assert!(!testlook!(look, "a ", 2));
        assert!(!testlook!(look, " a ", 0));
        assert!(!testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃b", 4));
        assert!(!testlook!(look, "𝛃 ", 5));
        assert!(!testlook!(look, " 𝛃 ", 0));
        assert!(!testlook!(look, " 𝛃 ", 6));
        assert!(!testlook!(look, "𝛃", 1));
        assert!(!testlook!(look, "𝛃", 2));
        assert!(!testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 1));
        assert!(!testlook!(look, "𝛃𐆀", 2));
        assert!(!testlook!(look, "𝛃𐆀", 3));
        assert!(!testlook!(look, "𝛃𐆀", 5));
        assert!(!testlook!(look, "𝛃𐆀", 6));
        assert!(!testlook!(look, "𝛃𐆀", 7));
        assert!(!testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    fn look_matches_word_ascii() {
        let look = Look::WordAscii;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(testlook!(look, "a", 0));
        assert!(testlook!(look, "a", 1));
        assert!(testlook!(look, "a ", 1));
        assert!(testlook!(look, " a ", 1));
        assert!(testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint. Since this is
        // an ASCII word boundary, none of these match.
        assert!(!testlook!(look, "𝛃", 0));
        assert!(!testlook!(look, "𝛃", 4));
        assert!(!testlook!(look, "𝛃 ", 4));
        assert!(!testlook!(look, " 𝛃 ", 1));
        assert!(!testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints. Again, since
        // this is an ASCII word boundary, none of these match.
        assert!(!testlook!(look, "𝛃𐆀", 0));
        assert!(!testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(!testlook!(look, "", 0));
        assert!(!testlook!(look, "ab", 1));
        assert!(!testlook!(look, "a ", 2));
        assert!(!testlook!(look, " a ", 0));
        assert!(!testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(testlook!(look, "𝛃b", 4));
        assert!(!testlook!(look, "𝛃 ", 5));
        assert!(!testlook!(look, " 𝛃 ", 0));
        assert!(!testlook!(look, " 𝛃 ", 6));
        assert!(!testlook!(look, "𝛃", 1));
        assert!(!testlook!(look, "𝛃", 2));
        assert!(!testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 1));
        assert!(!testlook!(look, "𝛃𐆀", 2));
        assert!(!testlook!(look, "𝛃𐆀", 3));
        assert!(!testlook!(look, "𝛃𐆀", 5));
        assert!(!testlook!(look, "𝛃𐆀", 6));
        assert!(!testlook!(look, "𝛃𐆀", 7));
        assert!(!testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    #[cfg(all(not(miri), feature = "unicode-word-boundary"))]
    fn look_matches_word_unicode_negate() {
        let look = Look::WordUnicodeNegate;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(!testlook!(look, "a", 0));
        assert!(!testlook!(look, "a", 1));
        assert!(!testlook!(look, "a ", 1));
        assert!(!testlook!(look, " a ", 1));
        assert!(!testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃", 0));
        assert!(!testlook!(look, "𝛃", 4));
        assert!(!testlook!(look, "𝛃 ", 4));
        assert!(!testlook!(look, " 𝛃 ", 1));
        assert!(!testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 0));
        assert!(!testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(testlook!(look, "", 0));
        assert!(testlook!(look, "ab", 1));
        assert!(testlook!(look, "a ", 2));
        assert!(testlook!(look, " a ", 0));
        assert!(testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(testlook!(look, "𝛃b", 4));
        assert!(testlook!(look, "𝛃 ", 5));
        assert!(testlook!(look, " 𝛃 ", 0));
        assert!(testlook!(look, " 𝛃 ", 6));
        // These don't match because they could otherwise return an offset that
        // splits the UTF-8 encoding of a codepoint.
        assert!(!testlook!(look, "𝛃", 1));
        assert!(!testlook!(look, "𝛃", 2));
        assert!(!testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints. These also don't
        // match because they could otherwise return an offset that splits the
        // UTF-8 encoding of a codepoint.
        assert!(!testlook!(look, "𝛃𐆀", 1));
        assert!(!testlook!(look, "𝛃𐆀", 2));
        assert!(!testlook!(look, "𝛃𐆀", 3));
        assert!(!testlook!(look, "𝛃𐆀", 5));
        assert!(!testlook!(look, "𝛃𐆀", 6));
        assert!(!testlook!(look, "𝛃𐆀", 7));
        // But this one does, since 𐆀 isn't a word codepoint, and 8 is the end
        // of the haystack. So the "end" of the haystack isn't a word and 𐆀
        // isn't a word, thus, \B matches.
        assert!(testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    fn look_matches_word_ascii_negate() {
        let look = Look::WordAsciiNegate;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(!testlook!(look, "a", 0));
        assert!(!testlook!(look, "a", 1));
        assert!(!testlook!(look, "a ", 1));
        assert!(!testlook!(look, " a ", 1));
        assert!(!testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint. Since this is
        // an ASCII word boundary, none of these match.
        assert!(testlook!(look, "𝛃", 0));
        assert!(testlook!(look, "𝛃", 4));
        assert!(testlook!(look, "𝛃 ", 4));
        assert!(testlook!(look, " 𝛃 ", 1));
        assert!(testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints. Again, since
        // this is an ASCII word boundary, none of these match.
        assert!(testlook!(look, "𝛃𐆀", 0));
        assert!(testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(testlook!(look, "", 0));
        assert!(testlook!(look, "ab", 1));
        assert!(testlook!(look, "a ", 2));
        assert!(testlook!(look, " a ", 0));
        assert!(testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃b", 4));
        assert!(testlook!(look, "𝛃 ", 5));
        assert!(testlook!(look, " 𝛃 ", 0));
        assert!(testlook!(look, " 𝛃 ", 6));
        assert!(testlook!(look, "𝛃", 1));
        assert!(testlook!(look, "𝛃", 2));
        assert!(testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(testlook!(look, "𝛃𐆀", 1));
        assert!(testlook!(look, "𝛃𐆀", 2));
        assert!(testlook!(look, "𝛃𐆀", 3));
        assert!(testlook!(look, "𝛃𐆀", 5));
        assert!(testlook!(look, "𝛃𐆀", 6));
        assert!(testlook!(look, "𝛃𐆀", 7));
        assert!(testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    fn look_matches_word_start_ascii() {
        let look = Look::WordStartAscii;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(testlook!(look, "a", 0));
        assert!(!testlook!(look, "a", 1));
        assert!(!testlook!(look, "a ", 1));
        assert!(testlook!(look, " a ", 1));
        assert!(!testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint. Since this is
        // an ASCII word boundary, none of these match.
        assert!(!testlook!(look, "𝛃", 0));
        assert!(!testlook!(look, "𝛃", 4));
        assert!(!testlook!(look, "𝛃 ", 4));
        assert!(!testlook!(look, " 𝛃 ", 1));
        assert!(!testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints. Again, since
        // this is an ASCII word boundary, none of these match.
        assert!(!testlook!(look, "𝛃𐆀", 0));
        assert!(!testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(!testlook!(look, "", 0));
        assert!(!testlook!(look, "ab", 1));
        assert!(!testlook!(look, "a ", 2));
        assert!(!testlook!(look, " a ", 0));
        assert!(!testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(testlook!(look, "𝛃b", 4));
        assert!(!testlook!(look, "b𝛃", 1));
        assert!(!testlook!(look, "𝛃 ", 5));
        assert!(!testlook!(look, " 𝛃 ", 0));
        assert!(!testlook!(look, " 𝛃 ", 6));
        assert!(!testlook!(look, "𝛃", 1));
        assert!(!testlook!(look, "𝛃", 2));
        assert!(!testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 1));
        assert!(!testlook!(look, "𝛃𐆀", 2));
        assert!(!testlook!(look, "𝛃𐆀", 3));
        assert!(!testlook!(look, "𝛃𐆀", 5));
        assert!(!testlook!(look, "𝛃𐆀", 6));
        assert!(!testlook!(look, "𝛃𐆀", 7));
        assert!(!testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    fn look_matches_word_end_ascii() {
        let look = Look::WordEndAscii;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(!testlook!(look, "a", 0));
        assert!(testlook!(look, "a", 1));
        assert!(testlook!(look, "a ", 1));
        assert!(!testlook!(look, " a ", 1));
        assert!(testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint. Since this is
        // an ASCII word boundary, none of these match.
        assert!(!testlook!(look, "𝛃", 0));
        assert!(!testlook!(look, "𝛃", 4));
        assert!(!testlook!(look, "𝛃 ", 4));
        assert!(!testlook!(look, " 𝛃 ", 1));
        assert!(!testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints. Again, since
        // this is an ASCII word boundary, none of these match.
        assert!(!testlook!(look, "𝛃𐆀", 0));
        assert!(!testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(!testlook!(look, "", 0));
        assert!(!testlook!(look, "ab", 1));
        assert!(!testlook!(look, "a ", 2));
        assert!(!testlook!(look, " a ", 0));
        assert!(!testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃b", 4));
        assert!(testlook!(look, "b𝛃", 1));
        assert!(!testlook!(look, "𝛃 ", 5));
        assert!(!testlook!(look, " 𝛃 ", 0));
        assert!(!testlook!(look, " 𝛃 ", 6));
        assert!(!testlook!(look, "𝛃", 1));
        assert!(!testlook!(look, "𝛃", 2));
        assert!(!testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 1));
        assert!(!testlook!(look, "𝛃𐆀", 2));
        assert!(!testlook!(look, "𝛃𐆀", 3));
        assert!(!testlook!(look, "𝛃𐆀", 5));
        assert!(!testlook!(look, "𝛃𐆀", 6));
        assert!(!testlook!(look, "𝛃𐆀", 7));
        assert!(!testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    #[cfg(all(not(miri), feature = "unicode-word-boundary"))]
    fn look_matches_word_start_unicode() {
        let look = Look::WordStartUnicode;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(testlook!(look, "a", 0));
        assert!(!testlook!(look, "a", 1));
        assert!(!testlook!(look, "a ", 1));
        assert!(testlook!(look, " a ", 1));
        assert!(!testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint.
        assert!(testlook!(look, "𝛃", 0));
        assert!(!testlook!(look, "𝛃", 4));
        assert!(!testlook!(look, "𝛃 ", 4));
        assert!(testlook!(look, " 𝛃 ", 1));
        assert!(!testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints.
        assert!(testlook!(look, "𝛃𐆀", 0));
        assert!(!testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(!testlook!(look, "", 0));
        assert!(!testlook!(look, "ab", 1));
        assert!(!testlook!(look, "a ", 2));
        assert!(!testlook!(look, " a ", 0));
        assert!(!testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃b", 4));
        assert!(!testlook!(look, "b𝛃", 1));
        assert!(!testlook!(look, "𝛃 ", 5));
        assert!(!testlook!(look, " 𝛃 ", 0));
        assert!(!testlook!(look, " 𝛃 ", 6));
        assert!(!testlook!(look, "𝛃", 1));
        assert!(!testlook!(look, "𝛃", 2));
        assert!(!testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 1));
        assert!(!testlook!(look, "𝛃𐆀", 2));
        assert!(!testlook!(look, "𝛃𐆀", 3));
        assert!(!testlook!(look, "𝛃𐆀", 5));
        assert!(!testlook!(look, "𝛃𐆀", 6));
        assert!(!testlook!(look, "𝛃𐆀", 7));
        assert!(!testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    #[cfg(all(not(miri), feature = "unicode-word-boundary"))]
    fn look_matches_word_end_unicode() {
        let look = Look::WordEndUnicode;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(!testlook!(look, "a", 0));
        assert!(testlook!(look, "a", 1));
        assert!(testlook!(look, "a ", 1));
        assert!(!testlook!(look, " a ", 1));
        assert!(testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃", 0));
        assert!(testlook!(look, "𝛃", 4));
        assert!(testlook!(look, "𝛃 ", 4));
        assert!(!testlook!(look, " 𝛃 ", 1));
        assert!(testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 0));
        assert!(testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(!testlook!(look, "", 0));
        assert!(!testlook!(look, "ab", 1));
        assert!(!testlook!(look, "a ", 2));
        assert!(!testlook!(look, " a ", 0));
        assert!(!testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃b", 4));
        assert!(!testlook!(look, "b𝛃", 1));
        assert!(!testlook!(look, "𝛃 ", 5));
        assert!(!testlook!(look, " 𝛃 ", 0));
        assert!(!testlook!(look, " 𝛃 ", 6));
        assert!(!testlook!(look, "𝛃", 1));
        assert!(!testlook!(look, "𝛃", 2));
        assert!(!testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 1));
        assert!(!testlook!(look, "𝛃𐆀", 2));
        assert!(!testlook!(look, "𝛃𐆀", 3));
        assert!(!testlook!(look, "𝛃𐆀", 5));
        assert!(!testlook!(look, "𝛃𐆀", 6));
        assert!(!testlook!(look, "𝛃𐆀", 7));
        assert!(!testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    fn look_matches_word_start_half_ascii() {
        let look = Look::WordStartHalfAscii;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(testlook!(look, "a", 0));
        assert!(!testlook!(look, "a", 1));
        assert!(!testlook!(look, "a ", 1));
        assert!(testlook!(look, " a ", 1));
        assert!(!testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint. Since this is
        // an ASCII word boundary, none of these match.
        assert!(testlook!(look, "𝛃", 0));
        assert!(testlook!(look, "𝛃", 4));
        assert!(testlook!(look, "𝛃 ", 4));
        assert!(testlook!(look, " 𝛃 ", 1));
        assert!(testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints. Again, since
        // this is an ASCII word boundary, none of these match.
        assert!(testlook!(look, "𝛃𐆀", 0));
        assert!(testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(testlook!(look, "", 0));
        assert!(!testlook!(look, "ab", 1));
        assert!(testlook!(look, "a ", 2));
        assert!(testlook!(look, " a ", 0));
        assert!(testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(testlook!(look, "𝛃b", 4));
        assert!(!testlook!(look, "b𝛃", 1));
        assert!(testlook!(look, "𝛃 ", 5));
        assert!(testlook!(look, " 𝛃 ", 0));
        assert!(testlook!(look, " 𝛃 ", 6));
        assert!(testlook!(look, "𝛃", 1));
        assert!(testlook!(look, "𝛃", 2));
        assert!(testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(testlook!(look, "𝛃𐆀", 1));
        assert!(testlook!(look, "𝛃𐆀", 2));
        assert!(testlook!(look, "𝛃𐆀", 3));
        assert!(testlook!(look, "𝛃𐆀", 5));
        assert!(testlook!(look, "𝛃𐆀", 6));
        assert!(testlook!(look, "𝛃𐆀", 7));
        assert!(testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    fn look_matches_word_end_half_ascii() {
        let look = Look::WordEndHalfAscii;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(!testlook!(look, "a", 0));
        assert!(testlook!(look, "a", 1));
        assert!(testlook!(look, "a ", 1));
        assert!(!testlook!(look, " a ", 1));
        assert!(testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint. Since this is
        // an ASCII word boundary, none of these match.
        assert!(testlook!(look, "𝛃", 0));
        assert!(testlook!(look, "𝛃", 4));
        assert!(testlook!(look, "𝛃 ", 4));
        assert!(testlook!(look, " 𝛃 ", 1));
        assert!(testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints. Again, since
        // this is an ASCII word boundary, none of these match.
        assert!(testlook!(look, "𝛃𐆀", 0));
        assert!(testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(testlook!(look, "", 0));
        assert!(!testlook!(look, "ab", 1));
        assert!(testlook!(look, "a ", 2));
        assert!(testlook!(look, " a ", 0));
        assert!(testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃b", 4));
        assert!(testlook!(look, "b𝛃", 1));
        assert!(testlook!(look, "𝛃 ", 5));
        assert!(testlook!(look, " 𝛃 ", 0));
        assert!(testlook!(look, " 𝛃 ", 6));
        assert!(testlook!(look, "𝛃", 1));
        assert!(testlook!(look, "𝛃", 2));
        assert!(testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(testlook!(look, "𝛃𐆀", 1));
        assert!(testlook!(look, "𝛃𐆀", 2));
        assert!(testlook!(look, "𝛃𐆀", 3));
        assert!(testlook!(look, "𝛃𐆀", 5));
        assert!(testlook!(look, "𝛃𐆀", 6));
        assert!(testlook!(look, "𝛃𐆀", 7));
        assert!(testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    #[cfg(all(not(miri), feature = "unicode-word-boundary"))]
    fn look_matches_word_start_half_unicode() {
        let look = Look::WordStartHalfUnicode;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(testlook!(look, "a", 0));
        assert!(!testlook!(look, "a", 1));
        assert!(!testlook!(look, "a ", 1));
        assert!(testlook!(look, " a ", 1));
        assert!(!testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint.
        assert!(testlook!(look, "𝛃", 0));
        assert!(!testlook!(look, "𝛃", 4));
        assert!(!testlook!(look, "𝛃 ", 4));
        assert!(testlook!(look, " 𝛃 ", 1));
        assert!(!testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints.
        assert!(testlook!(look, "𝛃𐆀", 0));
        assert!(!testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(testlook!(look, "", 0));
        assert!(!testlook!(look, "ab", 1));
        assert!(testlook!(look, "a ", 2));
        assert!(testlook!(look, " a ", 0));
        assert!(testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃b", 4));
        assert!(!testlook!(look, "b𝛃", 1));
        assert!(testlook!(look, "𝛃 ", 5));
        assert!(testlook!(look, " 𝛃 ", 0));
        assert!(testlook!(look, " 𝛃 ", 6));
        assert!(!testlook!(look, "𝛃", 1));
        assert!(!testlook!(look, "𝛃", 2));
        assert!(!testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 1));
        assert!(!testlook!(look, "𝛃𐆀", 2));
        assert!(!testlook!(look, "𝛃𐆀", 3));
        assert!(!testlook!(look, "𝛃𐆀", 5));
        assert!(!testlook!(look, "𝛃𐆀", 6));
        assert!(!testlook!(look, "𝛃𐆀", 7));
        assert!(testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    #[cfg(all(not(miri), feature = "unicode-word-boundary"))]
    fn look_matches_word_end_half_unicode() {
        let look = Look::WordEndHalfUnicode;

        // \xF0\x9D\x9B\x83 = 𝛃 (in \w)
        // \xF0\x90\x86\x80 = 𐆀 (not in \w)

        // Simple ASCII word boundaries.
        assert!(!testlook!(look, "a", 0));
        assert!(testlook!(look, "a", 1));
        assert!(testlook!(look, "a ", 1));
        assert!(!testlook!(look, " a ", 1));
        assert!(testlook!(look, " a ", 2));

        // Unicode word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃", 0));
        assert!(testlook!(look, "𝛃", 4));
        assert!(testlook!(look, "𝛃 ", 4));
        assert!(!testlook!(look, " 𝛃 ", 1));
        assert!(testlook!(look, " 𝛃 ", 5));

        // Unicode word boundaries between non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 0));
        assert!(testlook!(look, "𝛃𐆀", 4));

        // Non word boundaries for ASCII.
        assert!(testlook!(look, "", 0));
        assert!(!testlook!(look, "ab", 1));
        assert!(testlook!(look, "a ", 2));
        assert!(testlook!(look, " a ", 0));
        assert!(testlook!(look, " a ", 3));

        // Non word boundaries with a non-ASCII codepoint.
        assert!(!testlook!(look, "𝛃b", 4));
        assert!(!testlook!(look, "b𝛃", 1));
        assert!(testlook!(look, "𝛃 ", 5));
        assert!(testlook!(look, " 𝛃 ", 0));
        assert!(testlook!(look, " 𝛃 ", 6));
        assert!(!testlook!(look, "𝛃", 1));
        assert!(!testlook!(look, "𝛃", 2));
        assert!(!testlook!(look, "𝛃", 3));

        // Non word boundaries with non-ASCII codepoints.
        assert!(!testlook!(look, "𝛃𐆀", 1));
        assert!(!testlook!(look, "𝛃𐆀", 2));
        assert!(!testlook!(look, "𝛃𐆀", 3));
        assert!(!testlook!(look, "𝛃𐆀", 5));
        assert!(!testlook!(look, "𝛃𐆀", 6));
        assert!(!testlook!(look, "𝛃𐆀", 7));
        assert!(testlook!(look, "𝛃𐆀", 8));
    }

    #[test]
    fn look_set() {
        let mut f = LookSet::default();
        assert!(!f.contains(Look::Start));
        assert!(!f.contains(Look::End));
        assert!(!f.contains(Look::StartLF));
        assert!(!f.contains(Look::EndLF));
        assert!(!f.contains(Look::WordUnicode));
        assert!(!f.contains(Look::WordUnicodeNegate));
        assert!(!f.contains(Look::WordAscii));
        assert!(!f.contains(Look::WordAsciiNegate));

        f = f.insert(Look::Start);
        assert!(f.contains(Look::Start));
        f = f.remove(Look::Start);
        assert!(!f.contains(Look::Start));

        f = f.insert(Look::End);
        assert!(f.contains(Look::End));
        f = f.remove(Look::End);
        assert!(!f.contains(Look::End));

        f = f.insert(Look::StartLF);
        assert!(f.contains(Look::StartLF));
        f = f.remove(Look::StartLF);
        assert!(!f.contains(Look::StartLF));

        f = f.insert(Look::EndLF);
        assert!(f.contains(Look::EndLF));
        f = f.remove(Look::EndLF);
        assert!(!f.contains(Look::EndLF));

        f = f.insert(Look::StartCRLF);
        assert!(f.contains(Look::StartCRLF));
        f = f.remove(Look::StartCRLF);
        assert!(!f.contains(Look::StartCRLF));

        f = f.insert(Look::EndCRLF);
        assert!(f.contains(Look::EndCRLF));
        f = f.remove(Look::EndCRLF);
        assert!(!f.contains(Look::EndCRLF));

        f = f.insert(Look::WordUnicode);
        assert!(f.contains(Look::WordUnicode));
        f = f.remove(Look::WordUnicode);
        assert!(!f.contains(Look::WordUnicode));

        f = f.insert(Look::WordUnicodeNegate);
        assert!(f.contains(Look::WordUnicodeNegate));
        f = f.remove(Look::WordUnicodeNegate);
        assert!(!f.contains(Look::WordUnicodeNegate));

        f = f.insert(Look::WordAscii);
        assert!(f.contains(Look::WordAscii));
        f = f.remove(Look::WordAscii);
        assert!(!f.contains(Look::WordAscii));

        f = f.insert(Look::WordAsciiNegate);
        assert!(f.contains(Look::WordAsciiNegate));
        f = f.remove(Look::WordAsciiNegate);
        assert!(!f.contains(Look::WordAsciiNegate));

        f = f.insert(Look::WordStartAscii);
        assert!(f.contains(Look::WordStartAscii));
        f = f.remove(Look::WordStartAscii);
        assert!(!f.contains(Look::WordStartAscii));

        f = f.insert(Look::WordEndAscii);
        assert!(f.contains(Look::WordEndAscii));
        f = f.remove(Look::WordEndAscii);
        assert!(!f.contains(Look::WordEndAscii));

        f = f.insert(Look::WordStartUnicode);
        assert!(f.contains(Look::WordStartUnicode));
        f = f.remove(Look::WordStartUnicode);
        assert!(!f.contains(Look::WordStartUnicode));

        f = f.insert(Look::WordEndUnicode);
        assert!(f.contains(Look::WordEndUnicode));
        f = f.remove(Look::WordEndUnicode);
        assert!(!f.contains(Look::WordEndUnicode));

        f = f.insert(Look::WordStartHalfAscii);
        assert!(f.contains(Look::WordStartHalfAscii));
        f = f.remove(Look::WordStartHalfAscii);
        assert!(!f.contains(Look::WordStartHalfAscii));

        f = f.insert(Look::WordEndHalfAscii);
        assert!(f.contains(Look::WordEndHalfAscii));
        f = f.remove(Look::WordEndHalfAscii);
        assert!(!f.contains(Look::WordEndHalfAscii));

        f = f.insert(Look::WordStartHalfUnicode);
        assert!(f.contains(Look::WordStartHalfUnicode));
        f = f.remove(Look::WordStartHalfUnicode);
        assert!(!f.contains(Look::WordStartHalfUnicode));

        f = f.insert(Look::WordEndHalfUnicode);
        assert!(f.contains(Look::WordEndHalfUnicode));
        f = f.remove(Look::WordEndHalfUnicode);
        assert!(!f.contains(Look::WordEndHalfUnicode));
    }

    #[test]
    fn look_set_iter() {
        let set = LookSet::empty();
        assert_eq!(0, set.iter().count());

        let set = LookSet::full();
        assert_eq!(18, set.iter().count());

        let set =
            LookSet::empty().insert(Look::StartLF).insert(Look::WordUnicode);
        assert_eq!(2, set.iter().count());

        let set = LookSet::empty().insert(Look::StartLF);
        assert_eq!(1, set.iter().count());

        let set = LookSet::empty().insert(Look::WordAsciiNegate);
        assert_eq!(1, set.iter().count());

        let set = LookSet::empty().insert(Look::WordEndHalfUnicode);
        assert_eq!(1, set.iter().count());
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn look_set_debug() {
        let res = alloc::format!("{:?}", LookSet::empty());
        assert_eq!("∅", res);
        let res = alloc::format!("{:?}", LookSet::full());
        assert_eq!("Az^$rRbB𝛃𝚩<>〈〉◁▷◀▶", res);
    }
}
