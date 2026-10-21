#[cfg(test)]
mod tests {
    use std::format;

    use super::*;

    #[test]
    fn certificate_debug() {
        assert_eq!(
            "CertificateDer(0x6162)",
            format!("{:?}", CertificateDer::from(b"ab".to_vec()))
        );
    }

    #[cfg(feature = "ring")]
    #[test]
    fn webpki_supported_algorithms_is_debug() {
        assert_eq!(
            "WebPkiSupportedAlgorithms { all: [ .. ], mapping: [ECDSA_NISTP384_SHA384, ECDSA_NISTP256_SHA256, ED25519, RSA_PSS_SHA512, RSA_PSS_SHA384, RSA_PSS_SHA256, RSA_PKCS1_SHA512, RSA_PKCS1_SHA384, RSA_PKCS1_SHA256] }",
            format!(
                "{:?}",
                crate::crypto::ring::default_provider().signature_verification_algorithms
            )
        );
    }
}
