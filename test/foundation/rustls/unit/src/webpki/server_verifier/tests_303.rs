#[cfg(test)]
#[macro_rules_attribute::apply(test_for_each_provider)]
mod tests {
    use std::prelude::v1::*;
    use std::{println, vec};

    use pki_types::pem::PemObject;
    use pki_types::{CertificateDer, CertificateRevocationListDer};

    use super::{VerifierBuilderError, WebPkiServerVerifier, provider};
    use crate::RootCertStore;
    use crate::sync::Arc;

    fn load_crls(crls_der: &[&[u8]]) -> Vec<CertificateRevocationListDer<'static>> {
        crls_der
            .iter()
            .map(|pem_bytes| CertificateRevocationListDer::from_pem_slice(pem_bytes).unwrap())
            .collect()
    }

    fn test_crls() -> Vec<CertificateRevocationListDer<'static>> {
        load_crls(&[
            include_bytes!("../../../test-ca/ecdsa-p256/client.revoked.crl.pem").as_slice(),
            include_bytes!("../../../test-ca/rsa-2048/client.revoked.crl.pem").as_slice(),
        ])
    }

    fn load_roots(roots_der: &[&[u8]]) -> Arc<RootCertStore> {
        let mut roots = RootCertStore::empty();
        roots_der.iter().for_each(|der| {
            roots
                .add(CertificateDer::from(der.to_vec()))
                .unwrap()
        });
        roots.into()
    }

    fn test_roots() -> Arc<RootCertStore> {
        load_roots(&[
            include_bytes!("../../../test-ca/ecdsa-p256/ca.der").as_slice(),
            include_bytes!("../../../test-ca/rsa-2048/ca.der").as_slice(),
        ])
    }

    #[test]
    fn test_with_invalid_crls() {
        // Trying to build a server verifier with invalid CRLs should error at build time.
        let result = WebPkiServerVerifier::builder_with_provider(
            test_roots(),
            provider::default_provider().into(),
        )
        .with_crls(vec![CertificateRevocationListDer::from(vec![0xFF])])
        .build();
        assert!(matches!(result, Err(VerifierBuilderError::InvalidCrl(_))));
    }

    #[test]
    fn test_with_crls_multiple_calls() {
        // We should be able to call `with_crls` on a server verifier multiple times.
        let initial_crls = test_crls();
        let extra_crls =
            load_crls(&[
                include_bytes!("../../../test-ca/eddsa/client.revoked.crl.pem").as_slice(),
            ]);

        let builder = WebPkiServerVerifier::builder_with_provider(
            test_roots(),
            provider::default_provider().into(),
        )
        .with_crls(initial_crls.clone())
        .with_crls(extra_crls.clone());

        // There should be the expected number of crls.
        assert_eq!(builder.crls.len(), initial_crls.len() + extra_crls.len());
        // The builder should be Debug.
        println!("{builder:?}");
        builder.build().unwrap();
    }

    #[test]
    fn test_builder_no_roots() {
        // Trying to create a server verifier builder with no trust anchors should fail at build time
        let result = WebPkiServerVerifier::builder_with_provider(
            RootCertStore::empty().into(),
            provider::default_provider().into(),
        )
        .build();
        assert!(matches!(result, Err(VerifierBuilderError::NoRootAnchors)));
    }

    #[test]
    fn test_server_verifier_ee_only() {
        // We should be able to build a server cert. verifier that only checks the EE cert.
        let builder = WebPkiServerVerifier::builder_with_provider(
            test_roots(),
            provider::default_provider().into(),
        )
        .only_check_end_entity_revocation();
        // The builder should be Debug.
        println!("{builder:?}");
        builder.build().unwrap();
    }

    #[test]
    fn test_server_verifier_allow_unknown() {
        // We should be able to build a server cert. verifier that allows unknown revocation
        // status.
        let builder = WebPkiServerVerifier::builder_with_provider(
            test_roots(),
            provider::default_provider().into(),
        )
        .allow_unknown_revocation_status();
        // The builder should be Debug.
        println!("{builder:?}");
        builder.build().unwrap();
    }

    #[test]
    fn test_server_verifier_allow_unknown_ee_only() {
        // We should be able to build a server cert. verifier that allows unknown revocation
        // status and only checks the EE cert.
        let builder = WebPkiServerVerifier::builder_with_provider(
            test_roots(),
            provider::default_provider().into(),
        )
        .allow_unknown_revocation_status()
        .only_check_end_entity_revocation();
        // The builder should be Debug.
        println!("{builder:?}");
        builder.build().unwrap();
    }

    #[test]
    fn test_server_verifier_enforce_expiration() {
        // We should be able to build a server cert. verifier that allows unknown revocation
        // status.
        let builder = WebPkiServerVerifier::builder_with_provider(
            test_roots(),
            provider::default_provider().into(),
        )
        .enforce_revocation_expiration();
        // The builder should be Debug.
        println!("{builder:?}");
        builder.build().unwrap();
    }
}
