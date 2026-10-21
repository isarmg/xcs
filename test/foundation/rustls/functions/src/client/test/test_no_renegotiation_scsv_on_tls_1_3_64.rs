#[test]
    fn test_no_renegotiation_scsv_on_tls_1_3() {
        let ch = client_hello_sent_for_config(
            ClientConfig::builder_with_provider(super::provider::default_provider().into())
                .with_protocol_versions(&[&version::TLS13])
                .unwrap()
                .with_root_certificates(roots())
                .with_no_client_auth(),
        )
        .unwrap();
        assert!(
            !ch.cipher_suites
                .contains(&CipherSuite::TLS_EMPTY_RENEGOTIATION_INFO_SCSV)
        );
    }
