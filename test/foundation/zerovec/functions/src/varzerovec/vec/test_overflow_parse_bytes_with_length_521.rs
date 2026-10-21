#[test]
fn test_overflow_parse_bytes_with_length() {
    use super::components::{Index32, VarZeroVecComponents};
    // Tests that you can't try to construct a VZV that is too big to store its length
    // as a u32.

    // We want to distinguish between "failed because of slice bounds" and "failed because of u32 overflow".
    // On 64-bit, 4 * 0x4000_0000 = 4GB.
    // If we had the u32 check, it would return Metadata error regardless of slice size.
    // If we didn't, it would only return Metadata error if slice size < 4GB.

    // We don't use a fully valid VZV here because we just want to ensure the first few lines of code
    // return the metadata error without panicking. Actually allocating and instantiating
    // a 4GB VZV candidate is very slow.

    let mut bytes = vec![0u8; 4];
    bytes.copy_from_slice(&0x4000_0001u32.to_le_bytes());
    let result = VarZeroVecComponents::<str, Index32>::parse_bytes(&bytes);
    assert!(matches!(result, Err(VarZeroVecFormatError::Metadata)));
}
