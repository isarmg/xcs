#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_be_bytes() {
        static TEST_CASES: &[(&[u8], Result<&[u8], error::Unspecified>)] = &[
            // An empty input isn't a number.
            (&[], Err(error::Unspecified)),
            // Zero is not positive.
            (&[0x00], Err(error::Unspecified)),
            // Minimum value. No leading zero required or allowed.
            (&[0x00, 0x01], Err(error::Unspecified)),
            (&[0x01], Ok(&[0x01])),
            // Maximum first byte. No leading zero required or allowed.
            (&[0xff], Ok(&[0xff])),
            (&[0x00, 0xff], Err(error::Unspecified)),
            // The last byte can be zero.
            (&[0x01, 0x00], Ok(&[0x01, 0x00])),
            (&[0x01, 0x00, 0x00], Ok(&[0x01, 0x00, 0x00])),
            // Having no zero bytes are also allowed.
            (&[0x01, 0x01], Ok(&[0x01, 0x01])),
            // A middle byte can be zero.
            (&[0x01, 0x00, 0x01], Ok(&[0x01, 0x00, 0x01])),
            (&[0x01, 0x01, 0x01], Ok(&[0x01, 0x01, 0x01])),
        ];
        for &(input, result) in TEST_CASES {
            let input = untrusted::Input::from(input);
            assert_eq!(
                Positive::from_be_bytes(input).map(|p| p.big_endian_without_leading_zero()),
                result
            );
        }
    }
}
