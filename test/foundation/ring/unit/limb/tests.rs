#[allow(clippy::useless_conversion)]
#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use cfg_if::cfg_if;

    const MAX: LeakyLimb = LeakyLimb::MAX;

    fn leak_in_test(a: LimbMask) -> bool {
        a.leak()
    }

    #[test]
    fn test_limbs_are_even() {
        static EVENS: &[&[LeakyLimb]] = &[
            &[],
            &[0],
            &[2],
            &[0, 0],
            &[2, 0],
            &[0, 1],
            &[0, 2],
            &[0, 3],
            &[0, 0, 0, 0, MAX],
        ];
        for even in EVENS {
            let even = &Vec::from_iter(even.iter().copied().map(Limb::from));
            assert!(matches!(
                limbs_reject_even_leak_bit(even),
                Err(error::Unspecified)
            ));
        }
        static ODDS: &[&[LeakyLimb]] = &[
            &[1],
            &[3],
            &[1, 0],
            &[3, 0],
            &[1, 1],
            &[1, 2],
            &[1, 3],
            &[1, 0, 0, 0, MAX],
        ];
        for odd in ODDS {
            let odd = &Vec::from_iter(odd.iter().copied().map(Limb::from));
            assert!(matches!(limbs_reject_even_leak_bit(odd), Ok(())));
        }
    }

    static ZEROES: &[&[LeakyLimb]] = &[
        &[],
        &[0],
        &[0, 0],
        &[0, 0, 0],
        &[0, 0, 0, 0],
        &[0, 0, 0, 0, 0],
        &[0, 0, 0, 0, 0, 0, 0],
        &[0, 0, 0, 0, 0, 0, 0, 0],
        &[0, 0, 0, 0, 0, 0, 0, 0, 0],
    ];

    static NONZEROES: &[&[LeakyLimb]] = &[
        &[1],
        &[0, 1],
        &[1, 1],
        &[1, 0, 0, 0],
        &[0, 1, 0, 0],
        &[0, 0, 1, 0],
        &[0, 0, 0, 1],
    ];

    #[test]
    fn test_limbs_are_zero() {
        for zero in ZEROES {
            let zero = &Vec::from_iter(zero.iter().copied().map(Limb::from));
            assert!(leak_in_test(limbs_are_zero(zero)));
        }
        for nonzero in NONZEROES {
            let nonzero = &Vec::from_iter(nonzero.iter().copied().map(Limb::from));
            assert!(!leak_in_test(limbs_are_zero(nonzero)));
        }
    }

    #[test]
    fn test_limbs_equal_limb() {
        // Equal
        static EQUAL: &[&[LeakyLimb]] = &[&[1], &[1, 0], &[1, 0, 0], &[1, 0, 0, 0, 0, 0, 0]];
        for a in EQUAL {
            let a = &Vec::from_iter(a.iter().copied().map(Limb::from));
            assert!(matches!(verify_limbs_equal_1_leak_bit(a), Ok(())));
        }

        // Unequal
        static UNEQUAL: &[&[LeakyLimb]] = &[
            &[0],
            &[2],
            &[3],
            &[MAX],
            &[0, 1],
            &[1, 1],
            &[0, 0, 0, 0, 0, 0, 0, 1],
            &[0, 0, 0, 0, 1, 0, 0, 0],
            &[0, 0, 0, 0, 1, 0, 0, 1],
            &[MAX, 1],
        ];
        for a in UNEQUAL {
            let a = &Vec::from_iter(a.iter().copied().map(Limb::from));
            assert!(matches!(
                verify_limbs_equal_1_leak_bit(a),
                Err(error::Unspecified)
            ));
        }
    }

    #[test]
    fn test_parse_big_endian_and_pad_consttime() {
        const LIMBS: usize = 4;

        {
            // Empty input.
            let inp = untrusted::Input::from(&[]);
            let mut result = [0; LIMBS].map(From::<LeakyLimb>::from);
            assert!(parse_big_endian_and_pad_consttime(inp, &mut result).is_err());
        }

        // The input is longer than will fit in the given number of limbs.
        {
            let inp = [1, 2, 3, 4, 5, 6, 7, 8, 9];
            let inp = untrusted::Input::from(&inp);
            let mut result = [0; 8 / LIMB_BYTES].map(From::<LeakyLimb>::from);
            assert!(parse_big_endian_and_pad_consttime(inp, &mut result[..]).is_err());
        }

        // Less than a full limb.
        {
            let inp = [0xfe];
            let inp = untrusted::Input::from(&inp);
            let mut result = [0; LIMBS].map(From::<LeakyLimb>::from);
            assert_eq!(
                Ok(()),
                parse_big_endian_and_pad_consttime(inp, &mut result[..])
            );
            assert_eq!(&[0xfe, 0, 0, 0], &result);
        }

        // A whole limb for 32-bit, half a limb for 64-bit.
        {
            let inp = [0xbe, 0xef, 0xf0, 0x0d];
            let inp = untrusted::Input::from(&inp);
            let mut result = [0; LIMBS].map(From::<LeakyLimb>::from);
            assert_eq!(Ok(()), parse_big_endian_and_pad_consttime(inp, &mut result));
            assert_eq!(&[0xbeeff00d, 0, 0, 0], &result);
        }

        cfg_if! {
            if #[cfg(target_pointer_width = "64")] {
                static TEST_CASES: &[(&[u8], &[Limb])] = &[
                    (&[1], &[1, 0]),
                    (&[1, 2], &[0x102, 0]),
                    (&[1, 2, 3], &[0x10203, 0]),
                    (&[1, 2, 3, 4], &[0x102_0304, 0]),
                    (&[1, 2, 3, 4, 5], &[0x1_0203_0405, 0]),
                    (&[1, 2, 3, 4, 5, 6], &[0x102_0304_0506, 0]),
                    (&[1, 2, 3, 4, 5, 6, 7], &[0x1_0203_0405_0607, 0]),
                    (&[1, 2, 3, 4, 5, 6, 7, 8], &[0x102_0304_0506_0708, 0]),
                    (&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[0x0203_0405_0607_0809, 0x1]),
                    (&[1, 2, 3, 4, 5, 6, 7, 8, 9, 0xa], &[0x0304_0506_0708_090a, 0x102]),
                    (&[1, 2, 3, 4, 5, 6, 7, 8, 9, 0xa, 0xb], &[0x0405_0607_0809_0a0b, 0x1_0203]),
                ];
                for (be_bytes, limbs) in TEST_CASES {
                    let mut buf = [0; 2];
                    parse_big_endian_and_pad_consttime(untrusted::Input::from(be_bytes), &mut buf)
                        .unwrap();
                    assert_eq!(limbs, &buf, "({be_bytes:x?}, {limbs:x?}");
                }
            } else if #[cfg(target_pointer_width = "32")] {
                static TEST_CASES: &[(&[u8], &[Limb])] = &[
                    (&[1], &[1, 0, 0]),
                    (&[1, 2], &[0x102, 0, 0]),
                    (&[1, 2, 3], &[0x10203, 0, 0]),
                    (&[1, 2, 3, 4], &[0x102_0304, 0, 0]),
                    (&[1, 2, 3, 4, 5], &[0x0203_0405, 0x1, 0]),
                    (&[1, 2, 3, 4, 5, 6], &[0x0304_0506, 0x102, 0]),
                    (&[1, 2, 3, 4, 5, 6, 7], &[0x0405_0607, 0x1_0203, 0]),
                    (&[1, 2, 3, 4, 5, 6, 7, 8], &[0x0506_0708, 0x102_0304, 0]),
                    (&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[0x0607_0809, 0x0203_0405, 0x1]),
                    (&[1, 2, 3, 4, 5, 6, 7, 8, 9, 0xa], &[0x0708_090a, 0x0304_0506, 0x102]),
                    (&[1, 2, 3, 4, 5, 6, 7, 8, 9, 0xa, 0xb], &[0x0809_0a0b, 0x0405_0607, 0x1_0203]),
                ];
                for (be_bytes, limbs) in TEST_CASES {
                    let mut buf = [0; 3];
                    parse_big_endian_and_pad_consttime(untrusted::Input::from(be_bytes), &mut buf)
                        .unwrap();
                    assert_eq!(limbs, &buf, "({be_bytes:x?}, {limbs:x?}");
                }
            } else {
                panic!("Unsupported target_pointer_width");
            }

            // XXX: This is a weak set of tests. TODO: expand it.
        }
    }

    #[test]
    fn test_big_endian_from_limbs_same_length() {
        #[cfg(target_pointer_width = "32")]
        let limbs = [
            0xbccddeef, 0x89900aab, 0x45566778, 0x01122334, 0xddeeff00, 0x99aabbcc, 0x55667788,
            0x11223344,
        ];

        #[cfg(target_pointer_width = "64")]
        let limbs = [
            0x8990_0aab_bccd_deef,
            0x0112_2334_4556_6778,
            0x99aa_bbcc_ddee_ff00,
            0x1122_3344_5566_7788,
        ];

        let limbs = limbs.map(From::<LeakyLimb>::from);

        let expected = [
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
            0xff, 0x00, 0x01, 0x12, 0x23, 0x34, 0x45, 0x56, 0x67, 0x78, 0x89, 0x90, 0x0a, 0xab,
            0xbc, 0xcd, 0xde, 0xef,
        ];

        let mut out = [0xabu8; 32];
        big_endian_from_limbs(&limbs[..], &mut out);
        assert_eq!(&out[..], &expected[..]);
    }

    #[should_panic]
    #[test]
    fn test_big_endian_from_limbs_fewer_limbs() {
        #[cfg(target_pointer_width = "32")]
        // Two fewer limbs.
        let limbs = [
            0xbccddeef, 0x89900aab, 0x45566778, 0x01122334, 0xddeeff00, 0x99aabbcc,
        ];

        // One fewer limb.
        #[cfg(target_pointer_width = "64")]
        let limbs = [
            0x8990_0aab_bccd_deef,
            0x0112_2334_4556_6778,
            0x99aa_bbcc_ddee_ff00,
        ];

        let limbs = limbs.map(From::<LeakyLimb>::from);

        let mut out = [0xabu8; 32];

        big_endian_from_limbs(&limbs[..], &mut out);
    }

    #[test]
    fn test_limbs_minimal_bits() {
        const ALL_ONES: LeakyLimb = LeakyLimb::MAX;
        static CASES: &[(&[LeakyLimb], usize)] = &[
            (&[], 0),
            (&[0], 0),
            (&[ALL_ONES], LIMB_BITS),
            (&[ALL_ONES, 0], LIMB_BITS),
            (&[ALL_ONES, 1], LIMB_BITS + 1),
            (&[0, 0], 0),
            (&[1, 0], 1),
            (&[0, 1], LIMB_BITS + 1),
            (&[0, ALL_ONES], 2 * LIMB_BITS),
            (&[ALL_ONES, ALL_ONES], 2 * LIMB_BITS),
            (&[ALL_ONES, ALL_ONES >> 1], 2 * LIMB_BITS - 1),
            (&[ALL_ONES, 0b100_0000], LIMB_BITS + 7),
            (&[ALL_ONES, 0b101_0000], LIMB_BITS + 7),
            (&[ALL_ONES, ALL_ONES >> 1], LIMB_BITS + (LIMB_BITS) - 1),
        ];
        for (limbs, bits) in CASES {
            let limbs = &Vec::from_iter(limbs.iter().copied().map(Limb::from));
            assert_eq!(limbs_minimal_bits(limbs).as_bits(), *bits);
        }
    }
}
