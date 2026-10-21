#[test]
fn test_char_range_at_surrogate_boundary() {
    use crate::prelude::*;

    // Ranges ending exactly at '\u{E000}' must not produce surrogate codepoints.
    let chars: Vec<char> = ('\u{D7FE}'..'\u{E000}').into_par_iter().collect();
    assert_eq!(chars, vec!['\u{D7FE}', '\u{D7FF}']);

    // Ranges starting at '\u{E000}' are entirely above surrogates.
    let chars: Vec<char> = ('\u{E000}'..'\u{E002}').into_par_iter().collect();
    assert_eq!(chars, vec!['\u{E000}', '\u{E001}']);

    // A range that spans the surrogate gap should skip it.
    let chars: Vec<char> = ('\u{D7FE}'..'\u{E002}').into_par_iter().collect();
    assert_eq!(chars, vec!['\u{D7FE}', '\u{D7FF}', '\u{E000}', '\u{E001}']);
}
