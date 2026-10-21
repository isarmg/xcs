#[cfg(test)]
mod tests {
    use alloc::format;

    use pki_types::{PrivatePkcs1KeyDer, PrivateSec1KeyDer};

    use super::*;

    #[test]
    fn can_load_ecdsa_nistp256_pkcs8() {
        let key =
            PrivatePkcs8KeyDer::from(&include_bytes!("../../testdata/nistp256key.pkcs8.der")[..]);
        assert!(any_eddsa_type(&key).is_err());
        let key = PrivateKeyDer::Pkcs8(key);
        assert!(any_supported_type(&key).is_ok());
        assert!(any_ecdsa_type(&key).is_ok());
    }

    #[test]
    fn can_load_ecdsa_nistp256_sec1() {
        let key = PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(
            &include_bytes!("../../testdata/nistp256key.der")[..],
        ));
        assert!(any_supported_type(&key).is_ok());
        assert!(any_ecdsa_type(&key).is_ok());
    }

    #[test]
    fn can_sign_ecdsa_nistp256() {
        let key = PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(
            &include_bytes!("../../testdata/nistp256key.der")[..],
        ));

        let k = any_supported_type(&key).unwrap();
        assert_eq!(format!("{k:?}"), "EcdsaSigningKey { algorithm: ECDSA }");
        assert_eq!(k.algorithm(), SignatureAlgorithm::ECDSA);

        assert!(
            k.choose_scheme(&[SignatureScheme::RSA_PKCS1_SHA256])
                .is_none()
        );
        assert!(
            k.choose_scheme(&[SignatureScheme::ECDSA_NISTP384_SHA384])
                .is_none()
        );
        let s = k
            .choose_scheme(&[SignatureScheme::ECDSA_NISTP256_SHA256])
            .unwrap();
        assert_eq!(
            format!("{s:?}"),
            "EcdsaSigner { scheme: ECDSA_NISTP256_SHA256 }"
        );
        assert_eq!(s.scheme(), SignatureScheme::ECDSA_NISTP256_SHA256);
        // nb. signature is variable length and asn.1-encoded
        assert!(
            s.sign(b"hello")
                .unwrap()
                .starts_with(&[0x30])
        );
    }

    #[test]
    fn can_load_ecdsa_nistp384_pkcs8() {
        let key =
            PrivatePkcs8KeyDer::from(&include_bytes!("../../testdata/nistp384key.pkcs8.der")[..]);
        assert!(any_eddsa_type(&key).is_err());
        let key = PrivateKeyDer::Pkcs8(key);
        assert!(any_supported_type(&key).is_ok());
        assert!(any_ecdsa_type(&key).is_ok());
    }

    #[test]
    fn can_load_ecdsa_nistp384_sec1() {
        let key = PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(
            &include_bytes!("../../testdata/nistp384key.der")[..],
        ));
        assert!(any_supported_type(&key).is_ok());
        assert!(any_ecdsa_type(&key).is_ok());
    }

    #[test]
    fn can_sign_ecdsa_nistp384() {
        let key = PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(
            &include_bytes!("../../testdata/nistp384key.der")[..],
        ));

        let k = any_supported_type(&key).unwrap();
        assert_eq!(format!("{k:?}"), "EcdsaSigningKey { algorithm: ECDSA }");
        assert_eq!(k.algorithm(), SignatureAlgorithm::ECDSA);

        assert!(
            k.choose_scheme(&[SignatureScheme::RSA_PKCS1_SHA256])
                .is_none()
        );
        assert!(
            k.choose_scheme(&[SignatureScheme::ECDSA_NISTP256_SHA256])
                .is_none()
        );
        let s = k
            .choose_scheme(&[SignatureScheme::ECDSA_NISTP384_SHA384])
            .unwrap();
        assert_eq!(
            format!("{s:?}"),
            "EcdsaSigner { scheme: ECDSA_NISTP384_SHA384 }"
        );
        assert_eq!(s.scheme(), SignatureScheme::ECDSA_NISTP384_SHA384);
        // nb. signature is variable length and asn.1-encoded
        assert!(
            s.sign(b"hello")
                .unwrap()
                .starts_with(&[0x30])
        );
    }

    #[test]
    fn can_load_eddsa_pkcs8() {
        let key = PrivatePkcs8KeyDer::from(&include_bytes!("../../testdata/eddsakey.der")[..]);
        assert!(any_eddsa_type(&key).is_ok());
        let key = PrivateKeyDer::Pkcs8(key);
        assert!(any_supported_type(&key).is_ok());
        assert!(any_ecdsa_type(&key).is_err());
    }

    #[test]
    fn can_sign_eddsa() {
        let key = PrivatePkcs8KeyDer::from(&include_bytes!("../../testdata/eddsakey.der")[..]);

        let k = any_eddsa_type(&key).unwrap();
        assert_eq!(format!("{k:?}"), "Ed25519SigningKey { algorithm: ED25519 }");
        assert_eq!(k.algorithm(), SignatureAlgorithm::ED25519);

        assert!(
            k.choose_scheme(&[SignatureScheme::RSA_PKCS1_SHA256])
                .is_none()
        );
        assert!(
            k.choose_scheme(&[SignatureScheme::ECDSA_NISTP256_SHA256])
                .is_none()
        );
        let s = k
            .choose_scheme(&[SignatureScheme::ED25519])
            .unwrap();
        assert_eq!(format!("{s:?}"), "Ed25519Signer { scheme: ED25519 }");
        assert_eq!(s.scheme(), SignatureScheme::ED25519);
        assert_eq!(s.sign(b"hello").unwrap().len(), 64);
    }

    #[test]
    fn can_load_rsa2048_pkcs8() {
        let key =
            PrivatePkcs8KeyDer::from(&include_bytes!("../../testdata/rsa2048key.pkcs8.der")[..]);
        assert!(any_eddsa_type(&key).is_err());
        let key = PrivateKeyDer::Pkcs8(key);
        assert!(any_supported_type(&key).is_ok());
        assert!(any_ecdsa_type(&key).is_err());
    }

    #[test]
    fn can_load_rsa2048_pkcs1() {
        let key = PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(
            &include_bytes!("../../testdata/rsa2048key.pkcs1.der")[..],
        ));
        assert!(any_supported_type(&key).is_ok());
        assert!(any_ecdsa_type(&key).is_err());
    }

    #[test]
    fn can_sign_rsa2048() {
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            &include_bytes!("../../testdata/rsa2048key.pkcs8.der")[..],
        ));

        let k = any_supported_type(&key).unwrap();
        assert_eq!(format!("{k:?}"), "RsaSigningKey { algorithm: RSA }");
        assert_eq!(k.algorithm(), SignatureAlgorithm::RSA);

        assert!(
            k.choose_scheme(&[SignatureScheme::ECDSA_NISTP256_SHA256])
                .is_none()
        );
        assert!(
            k.choose_scheme(&[SignatureScheme::ED25519])
                .is_none()
        );

        let s = k
            .choose_scheme(&[SignatureScheme::RSA_PSS_SHA256])
            .unwrap();
        assert_eq!(format!("{s:?}"), "RsaSigner { scheme: RSA_PSS_SHA256 }");
        assert_eq!(s.scheme(), SignatureScheme::RSA_PSS_SHA256);
        assert_eq!(s.sign(b"hello").unwrap().len(), 256);

        for scheme in &[
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
        ] {
            k.choose_scheme(&[*scheme]).unwrap();
        }
    }

    #[test]
    fn cannot_load_invalid_pkcs8_encoding() {
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(&b"invalid"[..]));
        assert_eq!(
            any_supported_type(&key).err(),
            Some(Error::General(
                "failed to parse private key as RSA, ECDSA, or EdDSA".into()
            ))
        );
        assert_eq!(
            any_ecdsa_type(&key).err(),
            Some(Error::General(
                "failed to parse ECDSA private key as PKCS#8 or SEC1".into()
            ))
        );
        assert_eq!(
            RsaSigningKey::new(&key).err(),
            Some(Error::General(
                "failed to parse RSA private key: InvalidEncoding".into()
            ))
        );
    }
}
