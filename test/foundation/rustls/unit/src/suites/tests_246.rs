#[cfg(test)]
#[macro_rules_attribute::apply(test_for_each_provider)]
mod tests {
    use std::println;

    use super::provider::tls13::*;

    #[test]
    fn test_scs_is_debug() {
        println!("{:?}", super::provider::ALL_CIPHER_SUITES);
    }

    #[test]
    fn test_can_resume_to() {
        assert!(
            TLS13_AES_128_GCM_SHA256
                .tls13()
                .unwrap()
                .can_resume_from(TLS13_CHACHA20_POLY1305_SHA256_INTERNAL)
                .is_some()
        );
        assert!(
            TLS13_AES_256_GCM_SHA384
                .tls13()
                .unwrap()
                .can_resume_from(TLS13_CHACHA20_POLY1305_SHA256_INTERNAL)
                .is_none()
        );
    }
}
