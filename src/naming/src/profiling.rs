//! Historical persisted-key baseline for repeatable migration checks and profiling.
use crate::{unicode::UnicodeNormalization, unicode16};

/// Materialize each original NFC, Unicode 16 full fold and NFC pipeline stage.
///
/// The profile example compares this baseline with the streamed `lookup_key`.
/// Keeping it inside naming avoids exposing the private normalization engine or
/// compiling another copy of the generated Unicode tables into the example.
pub fn materialized_key(name: &str) -> String {
    let nfc = name.nfc().collect::<String>();
    let mut folded = String::new();
    for c in nfc.chars() {
        match unicode16::FOLD.binary_search_by_key(&(c as u32), |(value, _)| *value) {
            Ok(index) => folded.push_str(unicode16::FOLD[index].1),
            Err(_) => folded.push(c),
        }
    }
    folded.nfc().collect()
}
