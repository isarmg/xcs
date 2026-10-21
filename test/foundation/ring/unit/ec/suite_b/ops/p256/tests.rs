#[cfg(test)]
mod tests {
    #[cfg(any(
        all(target_arch = "aarch64", target_endian = "little"),
        target_arch = "x86_64"
    ))]
    #[test]
    fn p256_point_mul_base_vartime_test() {
        use super::{super::tests::point_mul_base_tests, *};
        point_mul_base_tests(
            &PRIVATE_KEY_OPS,
            point_mul_base_vartime,
            test_vector_file!("../../../../../fixtures/src/ec/suite_b/ops/p256_point_mul_base_tests.txt"),
        );
    }
}
