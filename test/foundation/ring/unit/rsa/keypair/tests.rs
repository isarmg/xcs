#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil as test;
    use alloc::vec;

    #[test]
    fn test_rsakeypair_private_exponentiate() {
        let cpu = cpu::features();
        test::run(
            test_vector_file!("../../../fixtures/src/rsa/keypair_private_exponentiate_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let key = test_case.consume_bytes("Key");
                let key = KeyPair::from_pkcs8(&key).unwrap();
                let test_cases = &[
                    test_case.consume_bytes("p"),
                    test_case.consume_bytes("p_plus_1"),
                    test_case.consume_bytes("p_minus_1"),
                    test_case.consume_bytes("q"),
                    test_case.consume_bytes("q_plus_1"),
                    test_case.consume_bytes("q_minus_1"),
                ];
                for test_case in test_cases {
                    // THe call to `elem_verify_equal_consttime` will cause
                    // `private_exponentiate` to fail if the computation is
                    // incorrect.
                    let mut padded = vec![0; key.public.modulus_len()];
                    let zeroes = padded.len() - test_case.len();
                    padded[zeroes..].copy_from_slice(test_case);
                    let _: bigint::Elem<_> = key.private_exponentiate(&padded, cpu).unwrap();
                }
                Ok(())
            },
        );
    }
}
