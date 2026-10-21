#[cfg(test)]
mod tests {
    use super::super::ops;
    use crate::testutil as test;
    use crate::{agreement, ec, limb};

    static SUPPORTED_SUITE_B_ALGS: [(&str, &agreement::Algorithm, &ec::Curve, &ops::CommonOps); 2] = [
        (
            "P-256",
            &agreement::ECDH_P256,
            &super::super::curve::P256,
            &ops::p256::COMMON_OPS,
        ),
        (
            "P-384",
            &agreement::ECDH_P384,
            &super::super::curve::P384,
            &ops::p384::COMMON_OPS,
        ),
    ];

    #[test]
    fn test_agreement_suite_b_ecdh_generate() {
        // Generates a string of bytes 0x00...00, which will always result in
        // a scalar value of zero.
        let random_00 = test::rand::FixedByteRandom { byte: 0x00 };

        // Generates a string of bytes 0xFF...FF, which will be larger than the
        // group order of any curve that is supported.
        let random_ff = test::rand::FixedByteRandom { byte: 0xff };

        for &(_, alg, curve, ops) in SUPPORTED_SUITE_B_ALGS.iter() {
            // Test that the private key value zero is rejected and that
            // `generate` gives up after a while of only getting zeros.
            assert!(agreement::EphemeralPrivateKey::generate(alg, &random_00).is_err());

            // Test that the private key value larger than the group order is
            // rejected and that `generate` gives up after a while of only
            // getting values larger than the group order.
            assert!(agreement::EphemeralPrivateKey::generate(alg, &random_ff).is_err());

            // Test that a private key value exactly equal to the group order
            // is rejected and that `generate` gives up after a while of only
            // getting that value from the PRNG.
            let mut n_bytes = [0u8; ec::SCALAR_MAX_BYTES];
            let num_bytes = curve.elem_scalar_seed_len;
            limb::big_endian_from_limbs(ops.n_limbs(), &mut n_bytes[..num_bytes]);
            {
                let n_bytes = &mut n_bytes[..num_bytes];
                let rng = test::rand::FixedSliceRandom { bytes: n_bytes };
                assert!(agreement::EphemeralPrivateKey::generate(alg, &rng).is_err());
            }

            // Test that a private key value exactly equal to the group order
            // minus 1 is accepted.
            let mut n_minus_1_bytes = n_bytes;
            {
                let n_minus_1_bytes = &mut n_minus_1_bytes[..num_bytes];
                n_minus_1_bytes[num_bytes - 1] -= 1;
                let rng = test::rand::FixedSliceRandom {
                    bytes: n_minus_1_bytes,
                };
                let key = agreement::EphemeralPrivateKey::generate(alg, &rng).unwrap();
                assert_eq!(n_minus_1_bytes, key.bytes_for_test());
            }

            // Test that n + 1 also fails.
            let mut n_plus_1_bytes = n_bytes;
            {
                let n_plus_1_bytes = &mut n_plus_1_bytes[..num_bytes];
                n_plus_1_bytes[num_bytes - 1] += 1;
                let rng = test::rand::FixedSliceRandom {
                    bytes: n_plus_1_bytes,
                };
                assert!(agreement::EphemeralPrivateKey::generate(alg, &rng).is_err());
            }

            // Test recovery from initial RNG failure. The first value will be
            // n, then n + 1, then zero, the next value will be n - 1, which
            // will be accepted.
            {
                let bytes = [
                    &n_bytes[..num_bytes],
                    &n_plus_1_bytes[..num_bytes],
                    &[0u8; ec::SCALAR_MAX_BYTES][..num_bytes],
                    &n_minus_1_bytes[..num_bytes],
                ];
                let rng = test::rand::FixedSliceSequenceRandom {
                    bytes: &bytes,
                    current: core::cell::UnsafeCell::new(0),
                };
                let key = agreement::EphemeralPrivateKey::generate(alg, &rng).unwrap();
                assert_eq!(&n_minus_1_bytes[..num_bytes], key.bytes_for_test());
            }
        }
    }
}
