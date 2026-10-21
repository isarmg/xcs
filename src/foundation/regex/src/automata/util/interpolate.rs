/*!
Provides routines for interpolating capture group references.

That is, if a replacement string contains references like `$foo` or `${foo1}`,
then they are replaced with the corresponding capture values for the groups
named `foo` and `foo1`, respectively. Similarly, syntax like `$1` and `${1}`
is supported as well, with `1` corresponding to a capture group index and not
a name.

This module provides the free functions [`string`] and [`bytes`], which
interpolate Rust Unicode strings and byte strings, respectively.

# Format

These routines support two different kinds of capture references: unbraced and
braced.

For the unbraced format, the format supported is `$ref` where `name` can be
any character in the class `[0-9A-Za-z_]`. `ref` is always the longest
possible parse. So for example, `$1a` corresponds to the capture group named
`1a` and not the capture group at index `1`. If `ref` matches `^[0-9]+$`, then
it is treated as a capture group index itself and not a name.

For the braced format, the format supported is `${ref}` where `ref` can be any
sequence of bytes except for `}`. If no closing brace occurs, then it is not
considered a capture reference. As with the unbraced format, if `ref` matches
`^[0-9]+$`, then it is treated as a capture group index and not a name.

The braced format is useful for exerting precise control over the name of the
capture reference. For example, `${1}a` corresponds to the capture group
reference `1` followed by the letter `a`, where as `$1a` (as mentioned above)
corresponds to the capture group reference `1a`. The braced format is also
useful for expressing capture group names that use characters not supported by
the unbraced format. For example, `${foo[bar].baz}` refers to the capture group
named `foo[bar].baz`.

If a capture group reference is found and it does not refer to a valid capture
group, then it will be replaced with the empty string.

To write a literal `$`, use `$$`.

To be clear, and as exhibited via the type signatures in the routines in this
module, it is impossible for a replacement string to be invalid. A replacement
string may not have the intended semantics, but the interpolation procedure
itself can never fail.
*/

use alloc::{string::String, vec::Vec};

use crate::automata::util::memchr::memchr;

/// Accepts a replacement string and interpolates capture references with their
/// corresponding values.
///
/// `append` should be a function that appends the string value of a capture
/// group at a particular index to the string given. If the capture group
/// index is invalid, then nothing should be appended.
///
/// `name_to_index` should be a function that maps a capture group name to a
/// capture group index. If the given name doesn't exist, then `None` should
/// be returned.
///
/// Finally, `dst` is where the final interpolated contents should be written.
/// If `replacement` contains no capture group references, then `dst` will be
/// equivalent to `replacement`.
///
/// See the [module documentation](self) for details about the format
/// supported.
///
/// # Example
///
/// ```
/// use regex::automata::util::interpolate;
///
/// let mut dst = String::new();
/// interpolate::string(
///     "foo $bar baz",
///     |index, dst| {
///         if index == 0 {
///             dst.push_str("BAR");
///         }
///     },
///     |name| {
///         if name == "bar" {
///             Some(0)
///         } else {
///             None
///         }
///     },
///     &mut dst,
/// );
/// assert_eq!("foo BAR baz", dst);
/// ```
pub fn string(
    mut replacement: &str,
    mut append: impl FnMut(usize, &mut String),
    mut name_to_index: impl FnMut(&str) -> Option<usize>,
    dst: &mut String,
) {
    while !replacement.is_empty() {
        match memchr(b'$', replacement.as_bytes()) {
            None => break,
            Some(i) => {
                dst.push_str(&replacement[..i]);
                replacement = &replacement[i..];
            }
        }
        // Handle escaping of '$'.
        if replacement.as_bytes().get(1).map_or(false, |&b| b == b'$') {
            dst.push_str("$");
            replacement = &replacement[2..];
            continue;
        }
        debug_assert!(!replacement.is_empty());
        let cap_ref = match find_cap_ref(replacement.as_bytes()) {
            Some(cap_ref) => cap_ref,
            None => {
                dst.push_str("$");
                replacement = &replacement[1..];
                continue;
            }
        };
        replacement = &replacement[cap_ref.end..];
        match cap_ref.cap {
            Ref::Number(i) => append(i, dst),
            Ref::Named(name) => {
                if let Some(i) = name_to_index(name) {
                    append(i, dst);
                }
            }
        }
    }
    dst.push_str(replacement);
}

/// Accepts a replacement byte string and interpolates capture references with
/// their corresponding values.
///
/// `append` should be a function that appends the byte string value of a
/// capture group at a particular index to the byte string given. If the
/// capture group index is invalid, then nothing should be appended.
///
/// `name_to_index` should be a function that maps a capture group name to a
/// capture group index. If the given name doesn't exist, then `None` should
/// be returned.
///
/// Finally, `dst` is where the final interpolated contents should be written.
/// If `replacement` contains no capture group references, then `dst` will be
/// equivalent to `replacement`.
///
/// See the [module documentation](self) for details about the format
/// supported.
///
/// # Example
///
/// ```
/// use regex::automata::util::interpolate;
///
/// let mut dst = vec![];
/// interpolate::bytes(
///     b"foo $bar baz",
///     |index, dst| {
///         if index == 0 {
///             dst.extend_from_slice(b"BAR");
///         }
///     },
///     |name| {
///         if name == "bar" {
///             Some(0)
///         } else {
///             None
///         }
///     },
///     &mut dst,
/// );
/// assert_eq!(&b"foo BAR baz"[..], dst);
/// ```
pub fn bytes(
    mut replacement: &[u8],
    mut append: impl FnMut(usize, &mut Vec<u8>),
    mut name_to_index: impl FnMut(&str) -> Option<usize>,
    dst: &mut Vec<u8>,
) {
    while !replacement.is_empty() {
        match memchr(b'$', replacement) {
            None => break,
            Some(i) => {
                dst.extend_from_slice(&replacement[..i]);
                replacement = &replacement[i..];
            }
        }
        // Handle escaping of '$'.
        if replacement.get(1).map_or(false, |&b| b == b'$') {
            dst.push(b'$');
            replacement = &replacement[2..];
            continue;
        }
        debug_assert!(!replacement.is_empty());
        let cap_ref = match find_cap_ref(replacement) {
            Some(cap_ref) => cap_ref,
            None => {
                dst.push(b'$');
                replacement = &replacement[1..];
                continue;
            }
        };
        replacement = &replacement[cap_ref.end..];
        match cap_ref.cap {
            Ref::Number(i) => append(i, dst),
            Ref::Named(name) => {
                if let Some(i) = name_to_index(name) {
                    append(i, dst);
                }
            }
        }
    }
    dst.extend_from_slice(replacement);
}

/// `CaptureRef` represents a reference to a capture group inside some text.
/// The reference is either a capture group name or a number.
///
/// It is also tagged with the position in the text following the
/// capture reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CaptureRef<'a> {
    cap: Ref<'a>,
    end: usize,
}

/// A reference to a capture group in some text.
///
/// e.g., `$2`, `$foo`, `${foo}`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Ref<'a> {
    Named(&'a str),
    Number(usize),
}

impl<'a> From<&'a str> for Ref<'a> {
    fn from(x: &'a str) -> Ref<'a> {
        Ref::Named(x)
    }
}

impl From<usize> for Ref<'static> {
    fn from(x: usize) -> Ref<'static> {
        Ref::Number(x)
    }
}

/// Parses a possible reference to a capture group name in the given text,
/// starting at the beginning of `replacement`.
///
/// If no such valid reference could be found, None is returned.
///
/// Note that this returns a "possible" reference because this routine doesn't
/// know whether the reference is to a valid group or not. If it winds up not
/// being a valid reference, then it should be replaced with the empty string.
fn find_cap_ref(replacement: &[u8]) -> Option<CaptureRef<'_>> {
    let mut i = 0;
    let rep: &[u8] = replacement;
    if rep.len() <= 1 || rep[0] != b'$' {
        return None;
    }
    i += 1;
    if rep[i] == b'{' {
        return find_cap_ref_braced(rep, i + 1);
    }
    let mut cap_end = i;
    while rep.get(cap_end).copied().map_or(false, is_valid_cap_letter) {
        cap_end += 1;
    }
    if cap_end == i {
        return None;
    }
    // We just verified that the range 0..cap_end is valid ASCII, so it must
    // therefore be valid UTF-8. If we really cared, we could avoid this UTF-8
    // check via an unchecked conversion or by parsing the number straight from
    // &[u8].
    let cap = core::str::from_utf8(&rep[i..cap_end])
        .expect("valid UTF-8 capture name");
    Some(CaptureRef {
        cap: match cap.parse::<usize>() {
            Ok(i) => Ref::Number(i),
            Err(_) => Ref::Named(cap),
        },
        end: cap_end,
    })
}

/// Looks for a braced reference, e.g., `${foo1}`. This assumes that an opening
/// brace has been found at `i-1` in `rep`. This then looks for a closing
/// brace and returns the capture reference within the brace.
fn find_cap_ref_braced(rep: &[u8], mut i: usize) -> Option<CaptureRef<'_>> {
    assert_eq!(b'{', rep[i.checked_sub(1).unwrap()]);
    let start = i;
    while rep.get(i).map_or(false, |&b| b != b'}') {
        i += 1;
    }
    if !rep.get(i).map_or(false, |&b| b == b'}') {
        return None;
    }
    // When looking at braced names, we don't put any restrictions on the name,
    // so it's possible it could be invalid UTF-8. But a capture group name
    // can never be invalid UTF-8, so if we have invalid UTF-8, then we can
    // safely return None.
    let cap = match core::str::from_utf8(&rep[start..i]) {
        Err(_) => return None,
        Ok(cap) => cap,
    };
    Some(CaptureRef {
        cap: match cap.parse::<usize>() {
            Ok(i) => Ref::Number(i),
            Err(_) => Ref::Named(cap),
        },
        end: i + 1,
    })
}

/// Returns true if and only if the given byte is allowed in a capture name
/// written in non-brace form.
fn is_valid_cap_letter(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_')
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/regex/automata/unit/src/util/interpolate/tests_327.rs"));
