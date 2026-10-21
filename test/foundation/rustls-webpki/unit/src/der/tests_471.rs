#[cfg(test)]
mod tests {
    use super::DerTypeId;
    #[cfg(feature = "alloc")]
    use alloc::vec::Vec;

    #[cfg(feature = "alloc")]
    #[test]
    fn test_asn1_wrap() {
        // Prepend stuff to `bytes` to put it in a DER SEQUENCE.
        let wrap_in_sequence = |bytes: &[u8]| super::asn1_wrap(super::Tag::Sequence, bytes);

        // Empty slice
        assert_eq!(vec![0x30, 0x00], wrap_in_sequence(&[]));

        // Small size
        assert_eq!(
            vec![0x30, 0x04, 0x00, 0x11, 0x22, 0x33],
            wrap_in_sequence(&[0x00, 0x11, 0x22, 0x33])
        );

        // Medium size
        let mut val = Vec::new();
        val.resize(255, 0x12);
        assert_eq!(
            vec![0x30, 0x81, 0xff, 0x12, 0x12, 0x12],
            wrap_in_sequence(&val)[..6]
        );

        // Large size
        let mut val = Vec::new();
        val.resize(4660, 0x12);
        wrap_in_sequence(&val);
        assert_eq!(
            vec![0x30, 0x82, 0x12, 0x34, 0x12, 0x12],
            wrap_in_sequence(&val)[..6]
        );

        // Huge size
        let mut val = Vec::new();
        val.resize(0xffff, 0x12);
        let result = wrap_in_sequence(&val);
        assert_eq!(vec![0x30, 0x82, 0xff, 0xff, 0x12, 0x12], result[..6]);
        assert_eq!(result.len(), 0xffff + 4);

        // Gigantic size
        let mut val = Vec::new();
        val.resize(0x100000, 0x12);
        let result = wrap_in_sequence(&val);
        assert_eq!(vec![0x30, 0x83, 0x10, 0x00, 0x00, 0x12, 0x12], result[..7]);
        assert_eq!(result.len(), 0x100000 + 5);

        // Ludicrous size
        let mut val = Vec::new();
        val.resize(0x1000000, 0x12);
        let result = wrap_in_sequence(&val);
        assert_eq!(
            vec![0x30, 0x84, 0x01, 0x00, 0x00, 0x00, 0x12, 0x12],
            result[..8]
        );
        assert_eq!(result.len(), 0x1000000 + 6);
    }

    #[test]
    fn test_optional_boolean() {
        use super::{Error, FromDer};

        // Empty input results in false
        assert!(!bool::from_der(&mut bytes_reader(&[])).unwrap());

        // Optional, so another data type results in false
        assert!(!bool::from_der(&mut bytes_reader(&[0x05, 0x00])).unwrap());

        // Only 0x00 and 0xff are accepted values
        assert_eq!(
            Err(Error::BadDer),
            bool::from_der(&mut bytes_reader(&[0x01, 0x01, 0x42]))
        );

        // True
        assert!(bool::from_der(&mut bytes_reader(&[0x01, 0x01, 0xff])).unwrap());

        // False
        assert!(!bool::from_der(&mut bytes_reader(&[0x01, 0x01, 0x00])).unwrap());
    }

    #[test]
    fn test_bit_string_with_no_unused_bits() {
        use super::{Error, bit_string_with_no_unused_bits};

        // Unexpected type
        assert_eq!(
            bit_string_with_no_unused_bits(&mut bytes_reader(&[0x01, 0x01, 0xff])).unwrap_err(),
            Error::TrailingData(DerTypeId::BitString),
        );

        // Unexpected nonexistent type
        assert_eq!(
            bit_string_with_no_unused_bits(&mut bytes_reader(&[0x42, 0xff, 0xff])).unwrap_err(),
            Error::TrailingData(DerTypeId::BitString),
        );

        // Unexpected empty input
        assert_eq!(
            bit_string_with_no_unused_bits(&mut bytes_reader(&[])).unwrap_err(),
            Error::TrailingData(DerTypeId::BitString),
        );

        // Valid input with non-zero unused bits
        assert_eq!(
            bit_string_with_no_unused_bits(&mut bytes_reader(&[0x03, 0x03, 0x04, 0x12, 0x34]))
                .unwrap_err(),
            Error::BadDer,
        );

        // Valid input
        assert_eq!(
            bit_string_with_no_unused_bits(&mut bytes_reader(&[0x03, 0x03, 0x00, 0x12, 0x34]))
                .unwrap()
                .as_slice_less_safe(),
            &[0x12, 0x34],
        );
    }

    fn bytes_reader(bytes: &[u8]) -> untrusted::Reader<'_> {
        untrusted::Reader::new(untrusted::Input::from(bytes))
    }

    #[test]
    fn read_tag_and_get_value_default_limit() {
        use super::{Error, read_tag_and_get_value};

        let inputs = &[
            // DER with short-form length encoded as three bytes.
            &[EXAMPLE_TAG, 0x83, 0xFF, 0xFF, 0xFF].as_slice(),
            // DER with short-form length encoded as four bytes.
            &[EXAMPLE_TAG, 0x84, 0xFF, 0xFF, 0xFF, 0xFF].as_slice(),
        ];

        for input in inputs {
            let mut bytes = untrusted::Reader::new(untrusted::Input::from(input));
            // read_tag_and_get_value should reject DER with encoded lengths larger than two
            // bytes as BadDer.
            assert!(matches!(
                read_tag_and_get_value(&mut bytes),
                Err(Error::BadDer)
            ));
        }
    }

    #[test]
    fn read_tag_and_get_value_limited_high_form() {
        use super::{Error, LONG_FORM_LEN_TWO_BYTES_MAX, read_tag_and_get_value_limited};

        let mut bytes = untrusted::Reader::new(untrusted::Input::from(&[0xFF]));
        // read_tag_and_get_value_limited_high_form should reject DER with "high tag number form" tags.
        assert!(matches!(
            read_tag_and_get_value_limited(&mut bytes, LONG_FORM_LEN_TWO_BYTES_MAX),
            Err(Error::BadDer)
        ));
    }

    #[test]
    fn read_tag_and_get_value_limited_non_canonical() {
        use super::{Error, LONG_FORM_LEN_TWO_BYTES_MAX, read_tag_and_get_value_limited};

        let inputs = &[
            // Two byte length, with expressed length < 128.
            &[EXAMPLE_TAG, 0x81, 0x01].as_slice(),
            // Three byte length, with expressed length < 256.
            &[EXAMPLE_TAG, 0x82, 0x00, 0x01].as_slice(),
            // Four byte length, with expressed length, < 65536.
            &[EXAMPLE_TAG, 0x83, 0x00, 0x00, 0x01].as_slice(),
            // Five byte length, with expressed length < 16777216.
            &[EXAMPLE_TAG, 0x84, 0x00, 0x00, 0x00, 0x01].as_slice(),
        ];

        for input in inputs {
            let mut bytes = untrusted::Reader::new(untrusted::Input::from(input));
            // read_tag_and_get_value_limited should reject DER with non-canonical lengths.
            assert!(matches!(
                read_tag_and_get_value_limited(&mut bytes, LONG_FORM_LEN_TWO_BYTES_MAX),
                Err(Error::BadDer)
            ));
        }
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn read_tag_and_get_value_limited_limits() {
        use super::{Error, read_tag_and_get_value_limited};

        let short_input = &[0xFF];
        let short_input_encoded = &[
            &[EXAMPLE_TAG],
            der_encode_length(short_input.len()).as_slice(),
            short_input,
        ]
        .concat();

        let long_input = &[1_u8; 65537];
        let long_input_encoded = &[
            &[EXAMPLE_TAG],
            der_encode_length(long_input.len()).as_slice(),
            long_input,
        ]
        .concat();

        struct Testcase<'a> {
            input: &'a [u8],
            limit: usize,
            err: Option<Error>,
        }

        let testcases = &[
            Testcase {
                input: short_input_encoded,
                limit: 1,
                err: Some(Error::BadDer),
            },
            Testcase {
                input: short_input_encoded,
                limit: short_input_encoded.len() + 1,
                err: None,
            },
            Testcase {
                input: long_input_encoded,
                limit: long_input.len(),
                err: Some(Error::BadDer),
            },
            Testcase {
                input: long_input_encoded,
                limit: long_input.len() + 1,
                err: None,
            },
        ];

        for tc in testcases {
            let mut bytes = untrusted::Reader::new(untrusted::Input::from(tc.input));

            let res = read_tag_and_get_value_limited(&mut bytes, tc.limit);
            match &tc.err {
                None => assert!(res.is_ok()),
                Some(e) => {
                    let actual = res.unwrap_err();
                    assert_eq!(&actual, e)
                }
            }
        }
    }

    #[allow(clippy::as_conversions)] // infallible.
    const EXAMPLE_TAG: u8 = super::Tag::Sequence as u8;

    #[cfg(feature = "alloc")]
    #[allow(clippy::as_conversions)] // test code.
    fn der_encode_length(length: usize) -> Vec<u8> {
        if length < 128 {
            vec![length as u8]
        } else {
            let mut encoded: Vec<u8> = Vec::new();
            let mut remaining_length = length;

            while remaining_length > 0 {
                let byte = (remaining_length & 0xFF) as u8;
                encoded.insert(0, byte);
                remaining_length >>= 8;
            }

            let length_octet = encoded.len() as u8 | 0x80;
            encoded.insert(0, length_octet);

            encoded
        }
    }

    #[test]
    fn misencoded_bit_string_flags() {
        use super::{Error, bit_string_flags};

        let bad_padding_example = untrusted::Input::from(&[
            0x08, // 8 bit of padding (illegal!).
            0x06, // 1 byte of bit flags asserting bits 5 and 6.
        ]);
        assert!(matches!(
            bit_string_flags(bad_padding_example),
            Err(Error::BadDer)
        ));

        let bad_padding_example = untrusted::Input::from(&[
            0x01, // 1 bit of padding.
                 // No flags value (illegal with padding!).
        ]);
        assert!(matches!(
            bit_string_flags(bad_padding_example),
            Err(Error::BadDer)
        ));

        // invalid padding for empty set
        for pad in 1..=255 {
            assert_eq!(
                bit_string_flags(untrusted::Input::from(&[pad])).err(),
                Some(Error::BadDer)
            );
        }
    }

    #[test]
    fn valid_bit_string_flags() {
        use super::bit_string_flags;

        let example_key_usage = untrusted::Input::from(&[
            0x01, // 1 bit of padding.
            0x06, // 1 byte of bit flags asserting bits 5 and 6.
        ]);
        let res = bit_string_flags(example_key_usage).unwrap();

        assert!(!res.bit_set(0));
        assert!(!res.bit_set(1));
        assert!(!res.bit_set(2));
        assert!(!res.bit_set(3));
        assert!(!res.bit_set(4));
        // NB: Bits 5 and 6 should be set.
        assert!(res.bit_set(5));
        assert!(res.bit_set(6));
        assert!(!res.bit_set(7));
        assert!(!res.bit_set(8));
        // Bits outside the range of values shouldn't be considered set.
        assert!(!res.bit_set(256));
    }

    #[test]
    fn empty_bit_string_flags() {
        let bs = super::bit_string_flags(untrusted::Input::from(&[0x00])).unwrap();

        // all bits are unset
        for b in 0..256 {
            assert!(!bs.bit_set(b));
        }
    }

    #[test]
    fn mispadded_bit_string_flags() {
        assert_eq!(
            super::bit_string_flags(untrusted::Input::from(&[0x04, 0xff])).err(),
            Some(super::Error::BadDer)
        );
    }

    #[test]
    fn test_small_nonnegative_integer() {
        use super::{Error, FromDer, Tag};

        for value in 0..=127 {
            let data = [Tag::Integer.into(), 1, value];
            let mut rd = untrusted::Reader::new(untrusted::Input::from(&data));
            assert_eq!(u8::from_der(&mut rd), Ok(value),);
        }

        for value in 128..=255 {
            let data = [Tag::Integer.into(), 2, 0x00, value];
            let mut rd = untrusted::Reader::new(untrusted::Input::from(&data));
            assert_eq!(u8::from_der(&mut rd), Ok(value),);
        }

        // not an integer
        assert_eq!(
            u8::from_der(&mut untrusted::Reader::new(untrusted::Input::from(&[
                Tag::Sequence.into(),
                1,
                1
            ]))),
            Err(Error::BadDer)
        );

        // negative
        assert_eq!(
            u8::from_der(&mut untrusted::Reader::new(untrusted::Input::from(&[
                Tag::Integer.into(),
                1,
                0xff
            ]))),
            Err(Error::BadDer)
        );

        // positive but too large
        assert_eq!(
            u8::from_der(&mut untrusted::Reader::new(untrusted::Input::from(&[
                Tag::Integer.into(),
                2,
                0x01,
                0x00
            ]))),
            Err(Error::BadDer)
        );

        // unnecessary leading zero
        assert_eq!(
            u8::from_der(&mut untrusted::Reader::new(untrusted::Input::from(&[
                Tag::Integer.into(),
                2,
                0x00,
                0x05
            ]))),
            Err(Error::BadDer)
        );

        // truncations
        assert_eq!(
            u8::from_der(&mut untrusted::Reader::new(untrusted::Input::from(&[]))),
            Err(Error::BadDer)
        );

        assert_eq!(
            u8::from_der(&mut untrusted::Reader::new(untrusted::Input::from(&[
                Tag::Integer.into(),
            ]))),
            Err(Error::BadDer)
        );

        assert_eq!(
            u8::from_der(&mut untrusted::Reader::new(untrusted::Input::from(&[
                Tag::Integer.into(),
                1,
            ]))),
            Err(Error::BadDer)
        );

        assert_eq!(
            u8::from_der(&mut untrusted::Reader::new(untrusted::Input::from(&[
                Tag::Integer.into(),
                2,
                0
            ]))),
            Err(Error::BadDer)
        );
    }
}
