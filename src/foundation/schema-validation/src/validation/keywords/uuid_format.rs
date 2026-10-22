//! Exact hyphenated UUID syntax, without decoding or version/variant restrictions.
//! Keep the hexadecimal lanes in one fixed array so the compiler can vectorize
//! the reduction on native and WASM targets. No architecture dispatch is needed.
pub(crate) fn is_valid(b: &[u8]) -> bool {
    let Ok(b) = <&[u8; 36]>::try_from(b) else {
        return false;
    };
    if [b[8], b[13], b[18], b[23]] != [b'-'; 4] {
        return false;
    }
    let hex = [
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[9], b[10], b[11], b[12], b[14], b[15],
        b[16], b[17], b[19], b[20], b[21], b[22], b[24], b[25], b[26], b[27], b[28], b[29], b[30],
        b[31], b[32], b[33], b[34], b[35],
    ];
    // Wrapping subtraction excludes bytes below each range as well as above it.
    // Setting ASCII bit 5 folds uppercase A-F into lowercase without admitting
    // non-ASCII bytes. Accumulate every lane, rather than short-circuiting.
    hex.iter().fold(0u8, |invalid, &b| {
        invalid | (!(b.wrapping_sub(b'0') < 10 || (b | 0x20).wrapping_sub(b'a') < 6) as u8)
    }) == 0
}
