#[cfg(test)]
mod tests {
    use super::digest_bytes_scalar;
    use crate::testutil as test;
    use crate::{cpu, digest, ec::suite_b::ops::*, limb};

    #[test]
    fn test() {
        let cpu = cpu::features();
        test::run(
            test_vector_file!("../../../../../fixtures/src/ec/suite_b/ecdsa/ecdsa_digest_scalar_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let curve_name = test_case.consume_string("Curve");
                let digest_name = test_case.consume_string("Digest");
                let input = test_case.consume_bytes("Input");
                let output = test_case.consume_bytes("Output");

                let (ops, digest_alg) = match (curve_name.as_str(), digest_name.as_str()) {
                    ("P-256", "SHA256") => (&p256::PUBLIC_SCALAR_OPS, &digest::SHA256),
                    ("P-256", "SHA384") => (&p256::PUBLIC_SCALAR_OPS, &digest::SHA384),
                    ("P-384", "SHA256") => (&p384::PUBLIC_SCALAR_OPS, &digest::SHA256),
                    ("P-384", "SHA384") => (&p384::PUBLIC_SCALAR_OPS, &digest::SHA384),
                    _ => {
                        panic!("Unsupported curve+digest: {}+{}", curve_name, digest_name);
                    }
                };
                let n = &ops.scalar_ops.scalar_modulus(cpu);

                assert_eq!(input.len(), digest_alg.output_len());
                assert_eq!(output.len(), ops.scalar_ops.scalar_bytes_len());
                assert_eq!(output.len(), n.bytes_len());

                let expected = scalar_parse_big_endian_variable(
                    n,
                    limb::AllowZero::Yes,
                    untrusted::Input::from(&output),
                )
                .unwrap();

                let actual = digest_bytes_scalar(n, &input);
                assert_eq!(
                    ops.scalar_ops.leak_limbs(&actual),
                    ops.scalar_ops.leak_limbs(&expected)
                );

                Ok(())
            },
        );
    }
}
