#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil as test;

    // Adapted from BoringSSL's crypto/poly1305/poly1305_test.cc.
    #[test]
    pub fn test_poly1305() {
        let cpu_features = cpu::features();
        test::run(
            test_vector_file!("../../../fixtures/src/aead/poly1305_test.txt"),
            |section, test_case| {
                assert_eq!(section, "");
                let key = test_case.consume_bytes("Key");
                let key: &[u8; KEY_LEN] = key.as_slice().try_into().unwrap();
                let input = test_case.consume_bytes("Input");
                let expected_mac = test_case.consume_bytes("MAC");
                let key = Key::new(*key);
                let Tag(actual_mac) = sign(key, &input, cpu_features);
                assert_eq!(expected_mac, actual_mac.as_ref());

                Ok(())
            },
        )
    }
}
