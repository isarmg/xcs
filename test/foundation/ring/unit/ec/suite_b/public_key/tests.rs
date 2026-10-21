#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu;
    use crate::testutil as test;

    #[test]
    fn parse_uncompressed_point_test() {
        let cpu = cpu::features();
        test::run(
            test_vector_file!("../../../../fixtures/src/ec/suite_b/suite_b_public_key_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let curve_name = test_case.consume_string("Curve");

                let public_key = test_case.consume_bytes("Q");
                let public_key = untrusted::Input::from(&public_key);
                let is_valid = test_case.consume_string("Result") == "P";

                let curve_ops = public_key_ops_from_curve_name(&curve_name);
                let q = &curve_ops.common.elem_modulus(cpu);

                let result = parse_uncompressed_point(curve_ops, q, public_key);
                assert_eq!(is_valid, result.is_ok());

                // TODO: Verify that we when we re-serialize the parsed (x, y), the
                // output is equal to the input.

                Ok(())
            },
        );
    }

    fn public_key_ops_from_curve_name(curve_name: &str) -> &'static PublicKeyOps {
        if curve_name == "P-256" {
            &p256::PUBLIC_KEY_OPS
        } else if curve_name == "P-384" {
            &p384::PUBLIC_KEY_OPS
        } else {
            panic!("Unsupported curve: {}", curve_name);
        }
    }
}
