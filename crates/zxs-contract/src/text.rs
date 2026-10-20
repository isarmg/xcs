//! Shared validation for new user-authored metadata. Historical readers may
//! retain explicit older limits without weakening current write admission.
use crate::{Error, Result};

pub const MAX_NOTE_BYTES: usize = 128;
pub const MAX_TAG_BYTES: usize = 128;
pub const MAX_FILE_TAGS: usize = 16;

pub fn validate_tag(tag: &str) -> Result<()> {
    if tag.is_empty()
        || tag.trim() != tag
        || tag.len() > MAX_TAG_BYTES
        || tag.chars().any(char::is_control)
    {
        return Err(Error::new(
            "INVALID_TAG",
            "tags must be nonempty, trimmed, control-free and at most 128 UTF-8 bytes",
        ));
    }
    Ok(())
}

/// Empty notes clear existing metadata. The limit counts UTF-8 bytes, and
/// control characters (including line breaks) are never stored by new writes.
pub fn validate_note(note: &str) -> Result<()> {
    if note.len() > MAX_NOTE_BYTES || note.chars().any(char::is_control) {
        return Err(Error::new(
            "INVALID_REQUEST",
            format!(
                "notes must be at most {MAX_NOTE_BYTES} UTF-8 bytes without line breaks or control characters"
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_share_byte_control_and_clear_semantics() {
        for note in [String::new(), "a".repeat(128), "汉".repeat(42) + "ab"] {
            validate_note(&note).unwrap();
        }
        for note in [
            "a".repeat(129),
            "汉".repeat(43),
            "line\nbreak".into(),
            "tab\there".into(),
            "\u{007f}".into(),
            "\u{0085}".into(),
        ] {
            assert_eq!(validate_note(&note).unwrap_err().code(), "INVALID_REQUEST");
        }
    }
}
