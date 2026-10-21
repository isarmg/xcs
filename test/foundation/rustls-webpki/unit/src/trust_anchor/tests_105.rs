#[cfg(test)]
mod tests {
    use rcgen::{CertificateParams, CustomExtension, KeyPair};

    use super::*;

    // OID 1.2.3.4 -- not under the id-ce arc, so `ExtensionOid::lookup` returns `None`.
    // This exercises the unknown-OID path in `remember_extension`.
    #[test]
    fn anchor_ignores_critical_extension_with_unknown_oid() {
        let der = cert_with_critical_extension(&[1, 2, 3, 4]);
        anchor_from_trusted_cert(&der)
            .expect("critical extension with unknown OID should be ignored for trust anchors");
    }

    // OID 2.5.29.99 -- under the id-ce arc, so `ExtensionOid::lookup` returns
    // `Some(Standard(99))`, but 99 isn't handled in `remember_cert_extension`.
    // This exercises the unsupported-extension path in `remember_cert_extension`.
    #[test]
    fn anchor_ignores_critical_extension_with_unknown_id_ce_oid() {
        let der = cert_with_critical_extension(&[2, 5, 29, 99]);
        anchor_from_trusted_cert(&der).expect(
            "critical id-ce extension with unknown OID should be ignored for trust anchors",
        );
    }

    fn cert_with_critical_extension(oid: &[u64]) -> CertificateDer<'static> {
        let mut params = CertificateParams::new(vec!["example.com".into()]).unwrap();
        let mut ext = CustomExtension::from_oid_content(oid, vec![1, 2]);
        ext.set_criticality(true);
        params.custom_extensions.push(ext);

        let key = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
        let cert = params.self_signed(&key).unwrap();
        CertificateDer::from(cert.der().to_vec())
    }
}
