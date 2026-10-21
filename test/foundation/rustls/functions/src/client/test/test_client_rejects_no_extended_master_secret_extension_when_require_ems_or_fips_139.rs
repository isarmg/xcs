#[cfg(feature = "tls12")]
    #[test]
    fn test_client_rejects_no_extended_master_secret_extension_when_require_ems_or_fips() {
        let mut config =
            ClientConfig::builder_with_provider(super::provider::default_provider().into())
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_root_certificates(roots())
                .with_no_client_auth();
        if config.provider.fips() {
            assert!(config.require_ems);
        } else {
            config.require_ems = true;
        }

        let config = Arc::new(config);
        let mut conn =
            ClientConnection::new(config.clone(), ServerName::try_from("localhost").unwrap())
                .unwrap();
        let mut sent = Vec::new();
        conn.write_tls(&mut sent).unwrap();

        let sh = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::handshake(HandshakeMessagePayload(
                HandshakePayload::ServerHello(ServerHelloPayload {
                    random: Random::new(config.provider.secure_random).unwrap(),
                    compression_method: Compression::Null,
                    cipher_suite: CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
                    legacy_version: ProtocolVersion::TLSv1_2,
                    session_id: SessionId::empty(),
                    extensions: Box::new(ServerExtensions::default()),
                }),
            )),
        };
        conn.read_tls(&mut sh.into_wire_bytes().as_slice())
            .unwrap();

        assert_eq!(
            conn.process_new_packets(),
            Err(PeerIncompatible::ExtendedMasterSecretExtensionRequired.into())
        );
    }
