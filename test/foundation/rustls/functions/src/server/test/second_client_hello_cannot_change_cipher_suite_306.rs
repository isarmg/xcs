#[test]
    fn second_client_hello_cannot_change_cipher_suite() {
        // RFC 9846 section 4.2.4 requires the server to negotiate the same cipher suite it
        // named in its HelloRetryRequest, and section 4.2.2 does not let the client vary its
        // offer, so a second hello that withdraws the retried suite cannot be honoured.
        let provider = CryptoProvider {
            cipher_suites: vec![
                super::provider::cipher_suite::TLS13_AES_128_GCM_SHA256,
                TLS13_AES_128_GCM_SHA256_ALT,
            ],
            ..super::provider::default_provider()
        };
        let config = ServerConfig::builder_with_provider(provider.into())
            .with_protocol_versions(&[&version::TLS13])
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(server_cert(), server_key())
            .unwrap();
        let mut conn = ServerConnection::new(config.into()).unwrap();

        let encode = |hello| {
            Message {
                version: ProtocolVersion::TLSv1_3,
                payload: MessagePayload::handshake(HandshakeMessagePayload(
                    HandshakePayload::ClientHello(hello),
                )),
            }
            .into_wire_bytes()
        };

        // this hello has no key share for a group we support, so it draws a
        // HelloRetryRequest naming `TLS13_AES_128_GCM_SHA256`.
        let mut first = minimal_client_hello();
        first.cipher_suites = vec![CipherSuite::TLS13_AES_128_GCM_SHA256];
        first.extensions.key_shares = Some(vec![]);
        conn.read_tls(&mut encode(first).as_slice())
            .unwrap();
        conn.process_new_packets().unwrap();

        // the second hello follows the retry, but offers only a different suite. It shares
        // the retried suite's hash, so the transcript stays valid and nothing else objects.
        let mut second = minimal_client_hello();
        second.cipher_suites = vec![TLS13_AES_128_GCM_SHA256_ALT.suite()];
        conn.read_tls(&mut encode(second).as_slice())
            .unwrap();

        assert_eq!(
            conn.process_new_packets().unwrap_err(),
            PeerMisbehaved::CipherSuiteDifferedOnRetry.into(),
        );
    }
