#[cfg(test)]
    mod tests {
        use super::*;
        use crate::server::ResolvesServerCert;

        #[test]
        fn test_resolvesservercertusingsni_requires_sni() {
            let rscsni = ResolvesServerCertUsingSni::new();
            assert!(
                rscsni
                    .resolve(ClientHello {
                        server_name: &None,
                        signature_schemes: &[],
                        alpn: None,
                        server_cert_types: None,
                        client_cert_types: None,
                        cipher_suites: &[],
                        certificate_authorities: None,
                        named_groups: None,
                    })
                    .is_none()
            );
        }

        #[test]
        fn test_resolvesservercertusingsni_handles_unknown_name() {
            let rscsni = ResolvesServerCertUsingSni::new();
            let name = DnsName::try_from("hello.com")
                .unwrap()
                .to_owned();
            assert!(
                rscsni
                    .resolve(ClientHello {
                        server_name: &Some(name),
                        signature_schemes: &[],
                        alpn: None,
                        server_cert_types: None,
                        client_cert_types: None,
                        cipher_suites: &[],
                        certificate_authorities: None,
                        named_groups: None,
                    })
                    .is_none()
            );
        }
    }
