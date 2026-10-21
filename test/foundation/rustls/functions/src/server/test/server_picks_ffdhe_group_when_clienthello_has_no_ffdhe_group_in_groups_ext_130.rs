#[cfg(feature = "tls12")]
    #[test]
    fn server_picks_ffdhe_group_when_clienthello_has_no_ffdhe_group_in_groups_ext() {
        let config = ServerConfig::builder_with_provider(ffdhe_provider().into())
            .with_protocol_versions(&[&version::TLS12])
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(server_cert(), server_key())
            .unwrap();

        let mut ch = minimal_client_hello();
        ch.cipher_suites
            .push(TLS_DHE_RSA_WITH_AES_128_GCM_SHA256.suite());

        server_chooses_ffdhe_group_for_client_hello(
            ServerConnection::new(config.into()).unwrap(),
            ch,
        );
    }
