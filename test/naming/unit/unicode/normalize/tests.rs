use super::compose_hangul;

// Regression test from a bugfix where we were composing an LV_Syllable with
// T_BASE directly. (We should only compose an LV_Syllable with a character
// in the range `T_BASE + 1 ..= T_LAST`.)
#[test]
fn test_hangul_composition() {
    assert_eq!(compose_hangul('\u{c8e0}', '\u{11a7}'), None);
}
