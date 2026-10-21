use crate::pod::POD;
use crate::Scalable;

/// An enum type which represents the case of ascii letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsciiCase {
    /// a-z are lower case letters.
    Lower,
    /// A-Z are upper case letters.
    Upper,
}

#[inline(always)]
fn convert_ascii_case<S: Scalable<V>, V: POD, const C: u8>(s: S, x: V) -> V {
    assert!(matches!(C, b'A' | b'a'));
    let x1 = s.u8xn_sub(x, s.u8xn_splat(C + 0x80));
    let x2 = s.i8xn_lt(x1, s.i8xn_splat(-0x80 + 26));
    let x3 = s.and(x2, s.u8xn_splat(0x20));
    s.xor(x, x3)
}

#[inline(always)]
pub fn to_ascii_lowercase<S: Scalable<V>, V: POD>(s: S, x: V) -> V {
    convert_ascii_case::<S, V, b'A'>(s, x)
}

#[inline(always)]
pub fn to_ascii_uppercase<S: Scalable<V>, V: POD>(s: S, x: V) -> V {
    convert_ascii_case::<S, V, b'a'>(s, x)
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/vsimd/unit/src/ascii/algorithm_32.rs"));
