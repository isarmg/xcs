#[cfg(test)]
mod tests {
    use super::*;
    use crate::msgs::enums::tests::{test_enum8, test_enum16};

    #[test]
    fn test_enums() {
        test_enum8::<SignatureAlgorithm>(SignatureAlgorithm::Anonymous, SignatureAlgorithm::ECDSA);
        test_enum8::<ContentType>(ContentType::ChangeCipherSpec, ContentType::Heartbeat);
        test_enum8::<HandshakeType>(HandshakeType::HelloRequest, HandshakeType::MessageHash);
        test_enum8::<AlertDescription>(
            AlertDescription::CloseNotify,
            AlertDescription::NoApplicationProtocol,
        );
        test_enum16::<CertificateCompressionAlgorithm>(
            CertificateCompressionAlgorithm::Zlib,
            CertificateCompressionAlgorithm::Zstd,
        );
        test_enum8::<CertificateType>(CertificateType::X509, CertificateType::RawPublicKey);
    }

    #[test]
    fn tls13_signature_restrictions() {
        // rsa-pkcs1 denied
        assert!(!SignatureScheme::RSA_PKCS1_SHA1.supported_in_tls13());
        assert!(!SignatureScheme::RSA_PKCS1_SHA256.supported_in_tls13());
        assert!(!SignatureScheme::RSA_PKCS1_SHA384.supported_in_tls13());
        assert!(!SignatureScheme::RSA_PKCS1_SHA512.supported_in_tls13());

        // dsa denied
        assert!(!SignatureScheme::from(0x0201).supported_in_tls13());
        assert!(!SignatureScheme::from(0x0202).supported_in_tls13());
        assert!(!SignatureScheme::from(0x0203).supported_in_tls13());
        assert!(!SignatureScheme::from(0x0204).supported_in_tls13());
        assert!(!SignatureScheme::from(0x0205).supported_in_tls13());
        assert!(!SignatureScheme::from(0x0206).supported_in_tls13());

        // common
        assert!(SignatureScheme::ED25519.supported_in_tls13());
        assert!(SignatureScheme::ED448.supported_in_tls13());
        assert!(SignatureScheme::RSA_PSS_SHA256.supported_in_tls13());
        assert!(SignatureScheme::RSA_PSS_SHA384.supported_in_tls13());
        assert!(SignatureScheme::RSA_PSS_SHA512.supported_in_tls13());

        // rsa_pss_rsae_*
        assert!(SignatureScheme::from(0x0804).supported_in_tls13());
        assert!(SignatureScheme::from(0x0805).supported_in_tls13());
        assert!(SignatureScheme::from(0x0806).supported_in_tls13());

        // ecdsa_brainpool*
        assert!(SignatureScheme::from(0x081a).supported_in_tls13());
        assert!(SignatureScheme::from(0x081b).supported_in_tls13());
        assert!(SignatureScheme::from(0x081c).supported_in_tls13());
    }
}
