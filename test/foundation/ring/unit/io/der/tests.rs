#[cfg(test)]
mod tests {
    use super::*;
    use crate::error;

    fn with_i<'a, F, R>(value: &'a [u8], f: F) -> Result<R, error::Unspecified>
    where
        F: FnOnce(&mut untrusted::Reader<'a>) -> Result<R, error::Unspecified>,
    {
        untrusted::Input::from(value).read_all(error::Unspecified, f)
    }

    static ZERO_INTEGER: &[u8] = &[0x02, 0x01, 0x00];

    static GOOD_POSITIVE_INTEGERS_SMALL: &[(&[u8], u8)] = &[
        (&[0x02, 0x01, 0x01], 0x01),
        (&[0x02, 0x01, 0x02], 0x02),
        (&[0x02, 0x01, 0x7e], 0x7e),
        (&[0x02, 0x01, 0x7f], 0x7f),
        // Values that need to have an 0x00 prefix to disambiguate them from
        // them from negative values.
        (&[0x02, 0x02, 0x00, 0x80], 0x80),
        (&[0x02, 0x02, 0x00, 0x81], 0x81),
        (&[0x02, 0x02, 0x00, 0xfe], 0xfe),
        (&[0x02, 0x02, 0x00, 0xff], 0xff),
    ];

    static GOOD_POSITIVE_INTEGERS_LARGE: &[(&[u8], &[u8])] = &[
        (&[0x02, 0x02, 0x01, 0x00], &[0x01, 0x00]),
        (&[0x02, 0x02, 0x02, 0x01], &[0x02, 0x01]),
        (&[0x02, 0x02, 0x7e, 0xfe], &[0x7e, 0xfe]),
        (&[0x02, 0x02, 0x7f, 0xff], &[0x7f, 0xff]),
        // Values that need to have an 0x00 prefix to disambiguate them from
        // them from negative values.
        (&[0x02, 0x03, 0x00, 0x80, 0x00], &[0x80, 0x00]),
        (&[0x02, 0x03, 0x00, 0x81, 0x01], &[0x81, 0x01]),
        (&[0x02, 0x03, 0x00, 0xfe, 0xfe], &[0xfe, 0xfe]),
        (&[0x02, 0x03, 0x00, 0xff, 0xff], &[0xff, 0xff]),
    ];

    static BAD_NONNEGATIVE_INTEGERS: &[&[u8]] = &[
        &[],           // At end of input
        &[0x02],       // Tag only
        &[0x02, 0x00], // Empty value
        // Length mismatch
        &[0x02, 0x00, 0x01],
        &[0x02, 0x01],
        // Would be valid if leading zero is ignored when comparing length.
        &[0x02, 0x01, 0x00, 0x01],
        &[0x02, 0x01, 0x01, 0x00], // Would be valid if last byte is ignored.
        &[0x02, 0x02, 0x01],
        // Values that are missing a necessary leading 0x00
        &[0x02, 0x01, 0x80],
        &[0x02, 0x01, 0x81],
        &[0x02, 0x01, 0xfe],
        &[0x02, 0x01, 0xff],
        // Values that have an unnecessary leading 0x00
        &[0x02, 0x02, 0x00, 0x00],
        &[0x02, 0x02, 0x00, 0x01],
        &[0x02, 0x02, 0x00, 0x02],
        &[0x02, 0x02, 0x00, 0x7e],
        &[0x02, 0x02, 0x00, 0x7f],
    ];

    #[test]
    fn test_small_nonnegative_integer() {
        let zero = (ZERO_INTEGER, 0x00);
        for &(test_in, test_out) in
            core::iter::once(&zero).chain(GOOD_POSITIVE_INTEGERS_SMALL.iter())
        {
            let result = with_i(test_in, |input| {
                assert_eq!(small_nonnegative_integer(input)?, test_out);
                Ok(())
            });
            assert_eq!(result, Ok(()));
        }
        for &test_in in BAD_NONNEGATIVE_INTEGERS
            .iter()
            .chain(GOOD_POSITIVE_INTEGERS_LARGE.iter().map(|(input, _)| input))
        {
            let result = with_i(test_in, small_nonnegative_integer);
            assert_eq!(result, Err(error::Unspecified));
        }
    }

    #[test]
    fn test_positive_integer() {
        for (test_in, test_out) in GOOD_POSITIVE_INTEGERS_SMALL
            .iter()
            .map(|(test_in, test_out)| (*test_in, core::slice::from_ref(test_out)))
            .chain(GOOD_POSITIVE_INTEGERS_LARGE.iter().copied())
        {
            let result = with_i(test_in, |input| {
                assert_eq!(
                    positive_integer(input)?.big_endian_without_leading_zero(),
                    test_out
                );
                Ok(())
            });
            assert_eq!(result, Ok(()))
        }
        for &test_in in core::iter::once(&ZERO_INTEGER).chain(BAD_NONNEGATIVE_INTEGERS.iter()) {
            let result = with_i(test_in, positive_integer);
            assert!(matches!(result, Err(error::Unspecified)));
        }
    }
}
