#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_u8() {
        let mut buffer = [0; 4];
        let mut pos = 1;
        let written = encode_fixed_uint_slice::<_, 8>(u8::MAX, &mut buffer, &mut pos);
        assert_eq!(3, pos);
        assert_eq!([0x00, 0xFF, 0x01, 0x00], buffer);
        assert_eq!(Some(2), written);
    }

    #[test]
    fn test_encode_u32() {
        let mut buffer = [0; 6];
        let mut pos = 1;
        let written = encode_fixed_uint_slice::<_, 32>(u32::MAX, &mut buffer, &mut pos);
        assert_eq!(6, pos);
        assert_eq!([0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F], buffer);
        assert_eq!(Some(5), written);
    }

    #[test]
    fn test_encode_u64_as_33_bits_2() {
        let mut buffer = [0; 6];
        let mut pos = 1;
        let written = encode_fixed_uint_slice::<_, 33>(2u64.pow(33) - 1, &mut buffer, &mut pos);
        let mut pos = 1;
        let value = decode_uint_slice::<u64, 33>(&buffer, &mut pos).unwrap();
        assert_eq!(8_589_934_592 - 1, value);
        assert_eq!(6, pos);
        assert_eq!([0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x1F], buffer);
        assert_eq!(Some(5), written);
    }

    #[test]
    fn test_encode_u64_as_33_bits_with_too_large_value() {
        let mut buffer = [0; 6];
        let mut pos = 1;
        let written = encode_fixed_uint_slice::<_, 33>(2u64.pow(34) - 1, &mut buffer, &mut pos);
        assert_eq!(1, pos);
        assert_eq!([0x00, 0x00, 0x00, 0x00, 0x00, 0x00], buffer);
        assert_eq!(None, written);
    }

    #[test]
    fn test_encode_u64() {
        let mut buffer = [0; 20];
        let mut pos = 1;
        let written = encode_fixed_uint_slice::<_, 64>(u64::MAX, &mut buffer, &mut pos);
        assert_eq!(11, pos);
        assert_eq!(Some(10), written);
    }

    #[test]
    fn test_decode_u32() {
        let input = [0xff, 0xff, 0xff, 0xff, 0x0f];
        let result = decode_u32(input);
        assert_eq!(result, Some((u32::MAX, 5)));

        let input = [0x00, 0x00, 0x00, 0x00, 0x00];
        let result = decode_u32(input);
        assert_eq!(result, Some((u32::MIN, 1)));

        // Valid but in-efficient way to encode 0.
        let input = [0x80, 0x80, 0x80, 0x80, 0x00];
        let result = decode_u32(input);
        assert_eq!(result, Some((u32::MIN, 5)));
    }

    #[test]
    fn test_decode_u32_errors() {
        // Maximum of 5 bytes encoding, the 0x80 bit must not be set.
        let input = [0xff, 0xff, 0xff, 0xff, 0x8f];
        let result = decode_u32(input);
        assert_eq!(result, None);

        // Parts of 0x1f (0x10) will be shifted out of the final value and lost.
        // This may too strict of a check since it could be ok.
        let input = [0xff, 0xff, 0xff, 0xff, 0x1f];
        let result = decode_u32(input);
        assert_eq!(result, None);
    }

    #[test]
    fn test_decode_u64() {
        let input = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01];
        let result = decode_u64(input);
        assert_eq!(result, Some((u64::MAX, 10)));

        let input = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let result = decode_u64(input);
        assert_eq!(result, Some((u64::MIN, 1)));

        // Valid but in-efficient way to encode 0.
        let input = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00];
        let result = decode_u64(input);
        assert_eq!(result, Some((u64::MIN, 10)));
    }

    #[test]
    fn test_decode_u64_errors() {
        // Maximum of 10 bytes encoding, the 0x80 bit must not be set in the final byte.
        let input = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x81];
        let result = decode_u64(input);
        assert_eq!(result, None);

        // 0x02 will be shifted out of the final value and lost.
        // This may too strict of a check since it could be ok.
        let input = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02];
        let result = decode_u64(input);
        assert_eq!(result, None);
    }

    #[test]
    fn test_decode_s32() {
        let input = [0xff, 0xff, 0xff, 0xff, 0x07];
        let result = decode_s32(input);
        assert_eq!(result, Some((i32::MAX, 5)));

        let input = [0x80, 0x80, 0x80, 0x80, 0x78];
        let result = decode_s32(input);
        assert_eq!(result, Some((i32::MIN, 5)));

        let input = [0x00, 0x00, 0x00, 0x00, 0x00];
        let result = decode_s32(input);
        assert_eq!(result, Some((0, 1)));

        // Valid but in-efficient way to encode 0.
        let input = [0x80, 0x80, 0x80, 0x80, 0x00];
        let result = decode_s32(input);
        assert_eq!(result, Some((0, 5)));

        let input = [0x40, 0x00, 0x00, 0x00, 0x00];
        let result = decode_s32(input);
        assert_eq!(result, Some((-64, 1)));

        // Valid but in-efficient way to encode -64.
        let input = [0xc0, 0x7f, 0x00, 0x00, 0x00];
        let result = decode_s32(input);
        assert_eq!(result, Some((-64, 2)));
    }

    #[test]
    fn test_decode_s32_errors() {
        // Maximum of 5 bytes encoding, the 0x80 bit must not be set in the final byte.
        let input = [0x80, 0x80, 0x80, 0x80, 0x80];
        let result = decode_s32(input);
        assert_eq!(result, None);

        // If the highest valid bit is set, it should be sign extended. (final byte should be 0x78)
        let input = [0x80, 0x80, 0x80, 0x80, 0x08];
        let result = decode_s32(input);
        assert_eq!(result, None);

        // If the highest valid bit is set, it should be sign extended. (final byte should be 0x78)
        let input = [0x80, 0x80, 0x80, 0x80, 0x38];
        let result = decode_s32(input);
        assert_eq!(result, None);
    }

    #[test]
    fn test_decode_s33() {
        decode_sint_arr!(decode_s33, i64, 33);

        let input = [0xff, 0xff, 0xff, 0xff, 0x0f];
        let result = decode_s33(input);
        assert_eq!(result, Some((i64::from(u32::MAX), 5)));

        let input = [0x80, 0x80, 0x80, 0x80, 0x70];
        let result = decode_s33(input);
        assert_eq!(result, Some((i64::from(i32::MIN) * 2, 5)));

        let input = [0x00, 0x00, 0x00, 0x00, 0x00];
        let result = decode_s33(input);
        assert_eq!(result, Some((0, 1)));

        // Valid but in-efficient way to encode 0.
        let input = [0x80, 0x80, 0x80, 0x80, 0x00];
        let result = decode_s33(input);
        assert_eq!(result, Some((0, 5)));

        let input = [0x40, 0x00, 0x00, 0x00, 0x00];
        let result = decode_s33(input);
        assert_eq!(result, Some((-64, 1)));

        // Valid but in-efficient way to encode -64.
        let input = [0xc0, 0x7f, 0x00, 0x00, 0x00];
        let result = decode_s33(input);
        assert_eq!(result, Some((-64, 2)));
    }

    #[test]
    fn test_decode_s33_errors() {
        decode_sint_arr!(decode_s33, i64, 33);

        // Maximum of 5 bytes encoding, the 0x80 bit must not be set in the final byte.
        let input = [0x80, 0x80, 0x80, 0x80, 0x80];
        let result = decode_s33(input);
        assert_eq!(result, None);

        // If the highest valid bit is set, it should be sign extended. (final byte should be 0x70)
        let input = [0x80, 0x80, 0x80, 0x80, 0x10];
        let result = decode_s33(input);
        assert_eq!(result, None);

        // If the highest valid bit is set, it should be sign extended. (final byte should be 0x70)
        let input = [0x80, 0x80, 0x80, 0x80, 0x30];
        let result = decode_s33(input);
        assert_eq!(result, None);
    }

    #[test]
    fn test_decode_s64() {
        let input = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00];
        let result = decode_s64(input);
        assert_eq!(result, Some((i64::MAX, 10)));

        let input = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x7f];
        let result = decode_s64(input);
        assert_eq!(result, Some((i64::MIN, 10)));

        let input = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let result = decode_s64(input);
        assert_eq!(result, Some((0, 1)));

        // Valid but in-efficient way to encode 0.
        let input = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00];
        let result = decode_s64(input);
        assert_eq!(result, Some((0, 10)));
    }

    #[test]
    fn test_decode_s64_errors() {
        // Maximum of 10 bytes encoding, the 0x80 bit must not be set in the final byte.
        let input = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80];
        let result = decode_s64(input);
        assert_eq!(result, None);

        // If the highest valid bit is set, it should be sign extended. (final byte should be 0x78)
        let input = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x08];
        let result = decode_s64(input);
        assert_eq!(result, None);

        // If the highest valid bit is set, it should be sign extended. (final byte should be 0x78)
        let input = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x28];
        let result = decode_s64(input);
        assert_eq!(result, None);
    }
}
