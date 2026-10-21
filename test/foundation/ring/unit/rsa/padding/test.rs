#[cfg(test)]
mod test {
    use super::*;
    use crate::testutil as test;
    use crate::{digest, error};
    use alloc::vec;

    #[test]
    fn test_pss_padding_verify() {
        test::run(
            test_vector_file!("../../../fixtures/src/rsa/rsa_pss_padding_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let digest_name = test_case.consume_string("Digest");
                let alg = match digest_name.as_ref() {
                    "SHA256" => &RSA_PSS_SHA256,
                    "SHA384" => &RSA_PSS_SHA384,
                    "SHA512" => &RSA_PSS_SHA512,
                    _ => panic!("Unsupported digest: {}", digest_name),
                };

                let msg = test_case.consume_bytes("Msg");
                let msg = untrusted::Input::from(&msg);
                let m_hash = digest::digest(alg.digest_alg(), msg.as_slice_less_safe());

                let encoded = test_case.consume_bytes("EM");
                let encoded = untrusted::Input::from(&encoded);

                // Salt is recomputed in verification algorithm.
                let _ = test_case.consume_bytes("Salt");

                let bit_len = test_case.consume_usize_bits("Len");
                let is_valid = test_case.consume_string("Result") == "P";

                let actual_result =
                    encoded.read_all(error::Unspecified, |m| alg.verify(m_hash, m, bit_len));
                assert_eq!(actual_result.is_ok(), is_valid);

                Ok(())
            },
        );
    }

    // Tests PSS encoding for various public modulus lengths.
    #[cfg(feature = "alloc")]
    #[test]
    fn test_pss_padding_encode() {
        test::run(
            test_vector_file!("../../../fixtures/src/rsa/rsa_pss_padding_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let digest_name = test_case.consume_string("Digest");
                let alg = match digest_name.as_ref() {
                    "SHA256" => &RSA_PSS_SHA256,
                    "SHA384" => &RSA_PSS_SHA384,
                    "SHA512" => &RSA_PSS_SHA512,
                    _ => panic!("Unsupported digest: {}", digest_name),
                };

                let msg = test_case.consume_bytes("Msg");
                let salt = test_case.consume_bytes("Salt");
                let encoded = test_case.consume_bytes("EM");
                let bit_len = test_case.consume_usize_bits("Len");
                let expected_result = test_case.consume_string("Result");

                // Only test the valid outputs
                if expected_result != "P" {
                    return Ok(());
                }

                let rng = test::rand::FixedSliceRandom { bytes: &salt };

                let mut m_out = vec![0u8; bit_len.as_usize_bytes_rounded_up()];
                let digest = digest::digest(alg.digest_alg(), &msg);
                #[allow(deprecated)]
                alg.encode(digest, &mut m_out, bit_len, &rng).unwrap();
                assert_eq!(m_out, encoded);

                Ok(())
            },
        );
    }
}
