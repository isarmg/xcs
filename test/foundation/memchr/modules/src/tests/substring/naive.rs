/*!
This module defines "naive" implementations of substring search.

These are sometimes useful to compare with "real" substring implementations.
The idea is that they are so simple that they are unlikely to be incorrect.
*/

/// Naively search forwards for the given needle in the given haystack.
pub(crate) fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let end = haystack.len().checked_sub(needle.len()).map_or(0, |i| i + 1);
    for i in 0..end {
        if needle == &haystack[i..i + needle.len()] {
            return Some(i);
        }
    }
    None
}

/// Naively search in reverse for the given needle in the given haystack.
pub(crate) fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let end = haystack.len().checked_sub(needle.len()).map_or(0, |i| i + 1);
    for i in (0..end).rev() {
        if needle == &haystack[i..i + needle.len()] {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/memchr/unit/src/tests/substring/naive/tests_30.rs"));
