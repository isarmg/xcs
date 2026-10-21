/// Computes the existing scalar-count help-layout width of `text`.
///
/// ANSI styling sequences are excluded. Each remaining Unicode scalar counts as
/// one column; this is not a Unicode grapheme or terminal display-width algorithm.
#[inline(never)]
pub(crate) fn display_width(text: &str) -> usize {
    let mut width = 0;

    let mut control_sequence = false;
    let control_terminate: char = 'm';

    for ch in text.chars() {
        if ch.is_ascii_control() {
            control_sequence = true;
        } else if control_sequence && ch == control_terminate {
            control_sequence = false;
            continue;
        }

        if !control_sequence {
            width += ch_width(ch);
        }
    }
    width
}



fn ch_width(_: char) -> usize {
    1
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/command-line/builder/unit/src/output/textwrap/core/tests_86.rs"));
