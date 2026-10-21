#[cfg(test)]
mod tests {
    use crate::testutil as test;
    use crate::{rand, signature};

    #[test]
    fn signature_ecdsa_sign_fixed_test() {
        let rng = rand::SystemRandom::new();

        test::run(
            test_vector_file!("../../../../../fixtures/src/ec/suite_b/ecdsa/ecdsa_sign_fixed_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let curve_name = test_case.consume_string("Curve");
                let digest_name = test_case.consume_string("Digest");
                let msg = test_case.consume_bytes("Msg");
                let d = test_case.consume_bytes("d");
                let q = test_case.consume_bytes("Q");
                let k = test_case.consume_bytes("k");

                let expected_result = test_case.consume_bytes("Sig");

                let alg = match (curve_name.as_str(), digest_name.as_str()) {
                    ("P-256", "SHA256") => &signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                    ("P-384", "SHA384") => &signature::ECDSA_P384_SHA384_FIXED_SIGNING,
                    _ => {
                        panic!("Unsupported curve+digest: {}+{}", curve_name, digest_name);
                    }
                };

                let private_key =
                    signature::EcdsaKeyPair::from_private_key_and_public_key(alg, &d, &q, &rng)
                        .unwrap();
                let rng = test::rand::FixedSliceRandom { bytes: &k };

                let actual_result = private_key
                    .sign_with_fixed_nonce_during_test(&rng, &msg)
                    .unwrap();

                assert_eq!(actual_result.as_ref(), &expected_result[..]);

                Ok(())
            },
        );
    }

    #[test]
    fn signature_ecdsa_sign_asn1_test() {
        let rng = rand::SystemRandom::new();

        test::run(
            test_vector_file!("../../../../../fixtures/src/ec/suite_b/ecdsa/ecdsa_sign_asn1_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let curve_name = test_case.consume_string("Curve");
                let digest_name = test_case.consume_string("Digest");
                let msg = test_case.consume_bytes("Msg");
                let d = test_case.consume_bytes("d");
                let q = test_case.consume_bytes("Q");
                let k = test_case.consume_bytes("k");

                let expected_result = test_case.consume_bytes("Sig");

                let alg = match (curve_name.as_str(), digest_name.as_str()) {
                    ("P-256", "SHA256") => &signature::ECDSA_P256_SHA256_ASN1_SIGNING,
                    ("P-384", "SHA384") => &signature::ECDSA_P384_SHA384_ASN1_SIGNING,
                    _ => {
                        panic!("Unsupported curve+digest: {}+{}", curve_name, digest_name);
                    }
                };

                let private_key =
                    signature::EcdsaKeyPair::from_private_key_and_public_key(alg, &d, &q, &rng)
                        .unwrap();
                let rng = test::rand::FixedSliceRandom { bytes: &k };

                let actual_result = private_key
                    .sign_with_fixed_nonce_during_test(&rng, &msg)
                    .unwrap();

                assert_eq!(actual_result.as_ref(), &expected_result[..]);

                Ok(())
            },
        );
    }
}
