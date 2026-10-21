#[cfg(feature = "tls12")]
    #[test]
    fn test_server_rejects_no_extended_master_secret_extension_when_require_ems_or_fips() {
        let provider = super::provider::default_provider();
        let mut config = ServerConfig::builder_with_provider(provider.into())
            .with_protocol_versions(&[&version::TLS12])
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(server_cert(), server_key())
            .unwrap();

        if config.provider.fips() {
            assert!(config.require_ems);
        } else {
            config.require_ems = true;
        }
        let mut conn = ServerConnection::new(config.into()).unwrap();

        let mut ch = minimal_client_hello();
        ch.extensions
            .extended_master_secret_request
            .take();
        let ch = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::handshake(HandshakeMessagePayload(
                HandshakePayload::ClientHello(ch),
            )),
        };
        conn.read_tls(&mut ch.into_wire_bytes().as_slice())
            .unwrap();

        assert_eq!(
            conn.process_new_packets(),
            Err(Error::PeerIncompatible(
                PeerIncompatible::ExtendedMasterSecretExtensionRequired
            ))
        );
    }
