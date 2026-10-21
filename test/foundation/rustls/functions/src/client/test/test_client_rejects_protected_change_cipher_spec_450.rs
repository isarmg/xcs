#[test]
    fn test_client_rejects_protected_change_cipher_spec() {
        let fake_server_crypto = Arc::new(FakeServerCrypto::new());
        let mut config = ClientConfig::builder_with_provider(x25519_provider().into())
            .with_protocol_versions(&[&version::TLS13])
            .unwrap()
            .with_root_certificates(roots())
            .with_no_client_auth();
        config.key_log = fake_server_crypto.clone();
        let mut conn =
            ClientConnection::new(config.into(), ServerName::try_from("localhost").unwrap())
                .unwrap();
        let mut sent = Vec::new();
        conn.write_tls(&mut sent).unwrap();

        let sh = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::handshake(HandshakeMessagePayload(
                HandshakePayload::ServerHello(ServerHelloPayload {
                    random: Random([0; 32]),
                    compression_method: Compression::Null,
                    cipher_suite: CipherSuite::TLS13_AES_128_GCM_SHA256,
                    legacy_version: ProtocolVersion::TLSv1_3,
                    session_id: SessionId::empty(),
                    extensions: Box::new(ServerExtensions {
                        key_share: Some(KeyShareEntry {
                            group: NamedGroup::X25519,
                            payload: PayloadU16::new(vec![0xaa; 32]),
                        }),
                        ..ServerExtensions::default()
                    }),
                }),
            )),
        };
        conn.read_tls(&mut sh.into_wire_bytes().as_slice())
            .unwrap();
        conn.process_new_packets().unwrap();

        // A change_cipher_spec protected under the handshake keys is not a
        // compatibility-mode CCS, and must not be dropped like one.
        let ccs = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::ChangeCipherSpec(ChangeCipherSpecPayload),
        };

        let mut encrypter = fake_server_crypto.server_handshake_encrypter();
        let enc_ccs = encrypter
            .encrypt(PlainMessage::from(ccs).borrow_outbound(), 0)
            .unwrap();
        conn.read_tls(&mut enc_ccs.encode().as_slice())
            .unwrap();
        assert_eq!(
            conn.process_new_packets().unwrap_err(),
            PeerMisbehaved::IllegalMiddleboxChangeCipherSpec.into()
        );
    }
