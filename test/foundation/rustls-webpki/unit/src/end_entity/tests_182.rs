#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;
    use crate::test_utils;
    use crate::test_utils::RCGEN_SIGNATURE_ALG;

    // This test reproduces https://github.com/rustls/webpki/issues/167 --- an
    // end-entity cert where the common name is a `PrintableString` rather than
    // a `UTF8String` cannot iterate over its subject alternative names.
    #[test]
    fn printable_string_common_name() {
        const DNS_NAME: &str = "test.example.com";

        let issuer = test_utils::make_issuer("Test");

        let ee_cert = {
            let mut params = test_utils::end_entity_params(vec![DNS_NAME.to_string()]);
            // construct a certificate that uses `PrintableString` as the
            // common name value, rather than `UTF8String`.
            params.distinguished_name.push(
                rcgen::DnType::CommonName,
                rcgen::DnValue::PrintableString(
                    rcgen::string::PrintableString::try_from("example.com").unwrap(),
                ),
            );
            params
                .signed_by(
                    &rcgen::KeyPair::generate_for(RCGEN_SIGNATURE_ALG).unwrap(),
                    &issuer,
                )
                .expect("failed to make ee cert (this is a test bug)")
        };

        expect_dns_name(ee_cert.der(), DNS_NAME);
    }

    // This test reproduces https://github.com/rustls/webpki/issues/167 --- an
    // end-entity cert where the common name is an empty SEQUENCE.
    #[test]
    fn empty_sequence_common_name() {
        let ee_cert_der = {
            // handcrafted cert DER produced using `ascii2der`, since `rcgen` is
            // unwilling to generate this particular weird cert.
            let bytes = include_bytes!("../tests/misc/empty_sequence_common_name.der");
            CertificateDer::from(&bytes[..])
        };
        expect_dns_name(&ee_cert_der, "example.com");
    }

    fn expect_dns_name(der: &CertificateDer<'_>, name: &str) {
        let cert =
            EndEntityCert::try_from(der).expect("should parse end entity certificate correctly");

        let mut names = cert.valid_dns_names();
        assert_eq!(names.next(), Some(name));
        assert_eq!(names.next(), None);
    }
}
