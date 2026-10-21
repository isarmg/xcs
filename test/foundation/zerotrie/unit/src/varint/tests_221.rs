#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestCase<'a> {
        bytes: &'a [u8],
        remainder: &'a [u8],
        value: usize,
    }
    static CASES: &[TestCase] = &[
        TestCase {
            bytes: &[0b00000000],
            remainder: &[],
            value: 0,
        },
        TestCase {
            bytes: &[0b00001010],
            remainder: &[],
            value: 10,
        },
        TestCase {
            bytes: &[0b00011111],
            remainder: &[],
            value: 31,
        },
        TestCase {
            bytes: &[0b00011111, 0b10101010],
            remainder: &[0b10101010],
            value: 31,
        },
        TestCase {
            bytes: &[0b00100000, 0b00000000],
            remainder: &[],
            value: 32,
        },
        TestCase {
            bytes: &[0b00100000, 0b00000001],
            remainder: &[],
            value: 33,
        },
        TestCase {
            bytes: &[0b00100000, 0b00100000],
            remainder: &[],
            value: 64,
        },
        TestCase {
            bytes: &[0x20, 0x44],
            remainder: &[],
            value: 100,
        },
        TestCase {
            bytes: &[0b00100000, 0b01111111],
            remainder: &[],
            value: 159,
        },
        TestCase {
            bytes: &[0b00100001, 0b00000000],
            remainder: &[],
            value: 160,
        },
        TestCase {
            bytes: &[0b00100001, 0b00000001],
            remainder: &[],
            value: 161,
        },
        TestCase {
            bytes: &[0x23, 0x54],
            remainder: &[],
            value: 500,
        },
        TestCase {
            bytes: &[0b00111111, 0b01111111],
            remainder: &[],
            value: 4127, // 32 + (1 << 12) - 1
        },
        TestCase {
            bytes: &[0b00100000, 0b10000000, 0b00000000],
            remainder: &[],
            value: 4128, // 32 + (1 << 12)
        },
        TestCase {
            bytes: &[0b00100000, 0b10000000, 0b00000001],
            remainder: &[],
            value: 4129, // 32 + (1 << 12) + 1
        },
        TestCase {
            bytes: &[0b00100000, 0b10000000, 0b01111111],
            remainder: &[],
            value: 4255, // 32 + (1 << 12) + 127
        },
        TestCase {
            bytes: &[0b00100000, 0b10000001, 0b00000000],
            remainder: &[],
            value: 4256, // 32 + (1 << 12) + 128
        },
        TestCase {
            bytes: &[0b00100000, 0b10000001, 0b00000001],
            remainder: &[],
            value: 4257, // 32 + (1 << 12) + 129
        },
        TestCase {
            bytes: &[0x20, 0x86, 0x68],
            remainder: &[],
            value: 5000,
        },
        TestCase {
            bytes: &[0b00100000, 0b11111111, 0b01111111],
            remainder: &[],
            value: 20511, // 32 + (1 << 12) + (1 << 14) - 1
        },
        TestCase {
            bytes: &[0b00100001, 0b10000000, 0b00000000],
            remainder: &[],
            value: 20512, // 32 + (1 << 12) + (1 << 14)
        },
        TestCase {
            bytes: &[0b00111111, 0b11111111, 0b01111111],
            remainder: &[],
            value: 528415, // 32 + (1 << 12) + (1 << 19) - 1
        },
        TestCase {
            bytes: &[0b00100000, 0b10000000, 0b10000000, 0b00000000],
            remainder: &[],
            value: 528416, // 32 + (1 << 12) + (1 << 19)
        },
        TestCase {
            bytes: &[0b00100000, 0b10000000, 0b10000000, 0b00000001],
            remainder: &[],
            value: 528417, // 32 + (1 << 12) + (1 << 19) + 1
        },
        TestCase {
            bytes: &[0b00111111, 0b11111111, 0b11111111, 0b01111111],
            remainder: &[],
            value: 67637279, // 32 + (1 << 12) + (1 << 19) + (1 << 26) - 1
        },
        TestCase {
            bytes: &[0b00100000, 0b10000000, 0b10000000, 0b10000000, 0b00000000],
            remainder: &[],
            value: 67637280, // 32 + (1 << 12) + (1 << 19) + (1 << 26)
        },
    ];

    #[test]
    fn test_read() {
        for cas in CASES {
            let recovered = read_varint_meta2(cas.bytes[0], &cas.bytes[1..]);
            assert_eq!(recovered, (cas.value, cas.remainder), "{cas:?}");
        }
    }

    #[test]
    fn test_read_write() {
        for cas in CASES {
            let reference_bytes = write_varint_reference(cas.value);
            assert_eq!(
                reference_bytes.len(),
                cas.bytes.len() - cas.remainder.len(),
                "{cas:?}"
            );
            assert_eq!(
                reference_bytes.as_slice(),
                &cas.bytes[0..reference_bytes.len()],
                "{cas:?}"
            );
            let recovered = read_varint_meta2(cas.bytes[0], &cas.bytes[1..]);
            assert_eq!(recovered, (cas.value, cas.remainder), "{cas:?}");
            let write_bytes = write_varint_meta2(cas.value);
            assert_eq!(
                reference_bytes.as_slice(),
                write_bytes.as_slice(),
                "{cas:?}"
            );
        }
    }

    #[test]
    fn test_roundtrip() {
        let mut i = 0usize;
        while i < MAX_VARINT {
            let bytes = write_varint_meta2(i);
            let recovered = read_varint_meta2(bytes.as_slice()[0], &bytes.as_slice()[1..]);
            assert_eq!(i, recovered.0, "{:?}", bytes.as_slice());
            i <<= 1;
            i += 1;
        }
    }

    #[test]
    fn test_extended_roundtrip() {
        let mut i = 0usize;
        while i < MAX_VARINT {
            let bytes = write_varint_meta3(i);
            let recovered = read_varint_meta3(bytes.as_slice()[0], &bytes.as_slice()[1..]);
            assert_eq!(i, recovered.0, "{:?}", bytes.as_slice());
            i <<= 1;
            i += 1;
        }
    }

    #[test]
    fn test_max() {
        let reference_bytes = write_varint_reference(MAX_VARINT);
        let write_bytes = write_varint_meta2(MAX_VARINT);
        assert_eq!(reference_bytes.len(), MAX_VARINT_LENGTH);
        assert_eq!(reference_bytes.as_slice(), write_bytes.as_slice());
        let subarray = write_bytes
            .as_const_slice()
            .get_subslice_or_panic(1, write_bytes.len());
        let (recovered_value, remainder) = read_varint_meta2(
            *write_bytes.as_const_slice().first().unwrap(),
            subarray.as_slice(),
        );
        assert!(remainder.is_empty());
        assert_eq!(recovered_value, MAX_VARINT);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(
            write_bytes.as_slice(),
            &[
                0b00100001, //
                0b11011111, //
                0b11011111, //
                0b11011111, //
                0b11011111, //
                0b11011111, //
                0b11011111, //
                0b11011111, //
                0b11011111, //
                0b01011111, //
            ]
        );
        #[cfg(target_pointer_width = "32")]
        assert_eq!(
            write_bytes.as_slice(),
            &[
                0b00101111, //
                0b11011111, //
                0b11011111, //
                0b11011111, //
                0b01011111, //
            ]
        );
    }

    #[test]
    fn text_extended_max() {
        let write_bytes = write_varint_meta3(MAX_VARINT);
        assert_eq!(write_bytes.len(), MAX_VARINT_LENGTH);
        let (lead, trailing) = write_bytes.as_slice().split_first().unwrap();
        let (recovered_value, remainder) = read_varint_meta3(*lead, trailing);
        assert!(remainder.is_empty());
        assert_eq!(recovered_value, MAX_VARINT);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(
            write_bytes.as_slice(),
            &[
                0b00010001, //
                0b11101111, //
                0b11101111, //
                0b11101111, //
                0b11101111, //
                0b11101111, //
                0b11101111, //
                0b11101111, //
                0b11101111, //
                0b01101111, //
            ]
        );
        #[cfg(target_pointer_width = "32")]
        assert_eq!(
            write_bytes.as_slice(),
            &[
                0b00011111, //
                0b11101111, //
                0b11101111, //
                0b11101111, //
                0b01101111, //
            ]
        );
    }

    #[test]
    fn test_latent_values() {
        // Same values documented in the module docs: M=2
        let m2 = read_varint_meta2;
        assert_eq!(m2(0, &[]).0, 0);
        assert_eq!(m2(0x20, &[0x00]).0, 32);
        assert_eq!(m2(0x20, &[0x80, 0x00]).0, 4128);
        assert_eq!(m2(0x20, &[0x80, 0x80, 0x00]).0, 528416);
        assert_eq!(m2(0x20, &[0x80, 0x80, 0x80, 0x00]).0, 67637280);

        // Same values documented in the module docs: M=3
        let m3 = read_varint_meta3;
        assert_eq!(m3(0, &[]).0, 0);
        assert_eq!(m3(0x10, &[0x00]).0, 16);
        assert_eq!(m3(0x10, &[0x80, 0x00]).0, 2064);
        assert_eq!(m3(0x10, &[0x80, 0x80, 0x00]).0, 264208);
        assert_eq!(m3(0x10, &[0x80, 0x80, 0x80, 0x00]).0, 33818640);
    }
}
