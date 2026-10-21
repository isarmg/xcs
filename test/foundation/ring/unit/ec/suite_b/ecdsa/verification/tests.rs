#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use crate::testutil as test;
    use alloc::{vec, vec::Vec};

    #[test]
    fn test_digest_based_test_vectors() {
        let cpu = cpu::features();
        test::run(
            test_vector_file!("../../../../../fixtures/crypto/fipsmodule/ecdsa/ecdsa_verify_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let curve_name = test_case.consume_string("Curve");

                let public_key = {
                    let mut public_key = vec![0x04];
                    public_key.extend(&test_case.consume_bytes("X"));
                    public_key.extend(&test_case.consume_bytes("Y"));
                    public_key
                };

                let digest = test_case.consume_bytes("Digest");

                let sig = {
                    let mut sig = Vec::new();
                    sig.extend(&test_case.consume_bytes("R"));
                    sig.extend(&test_case.consume_bytes("S"));
                    sig
                };

                let invalid = test_case.consume_optional_string("Invalid");

                let alg = match curve_name.as_str() {
                    "P-256" => &ECDSA_P256_SHA256_FIXED,
                    "P-384" => &ECDSA_P384_SHA384_FIXED,
                    _ => {
                        panic!("Unsupported curve: {}", curve_name);
                    }
                };
                let n = &alg.ops.scalar_ops.scalar_modulus(cpu);

                let digest = super::super::digest_scalar::digest_bytes_scalar(n, &digest[..]);
                let actual_result = alg.verify_digest(
                    untrusted::Input::from(&public_key[..]),
                    digest,
                    untrusted::Input::from(&sig[..]),
                );
                assert_eq!(actual_result.is_ok(), invalid.is_none());

                Ok(())
            },
        );
    }
}
