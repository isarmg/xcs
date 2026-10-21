#[test]
    fn second_client_hello_cannot_withdraw_psk_offer() {
        // Per RFC 9846 section 4.2.2, dropping a PreSharedKey offer is not one of the
        // changes a client may make after a HelloRetryRequest.
        let config =
            ServerConfig::builder_with_provider(super::provider::default_provider().into())
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

        // this hello offers a PSK, but no key share for a group we support, so
        // it draws a HelloRetryRequest.
        let mut first = minimal_client_hello();
        first.extensions.key_shares = Some(vec![]);
        first.extensions.preshared_key_modes = Some(PskKeyExchangeModes {
            psk_dhe: true,
            psk: false,
        });
        first.extensions.preshared_key_offer = Some(PresharedKeyOffer::new(
            PresharedKeyIdentity::new(vec![0u8; 16], 0),
            vec![0u8; 32],
        ));
        conn.read_tls(&mut encode(first).as_slice())
            .unwrap();
        conn.process_new_packets().unwrap();

        // the second hello follows the retry, but drops the PSK offer entirely.
        let mut second = minimal_client_hello();
        second.extensions.preshared_key_modes = Some(PskKeyExchangeModes {
            psk_dhe: true,
            psk: false,
        });
        conn.read_tls(&mut encode(second).as_slice())
            .unwrap();

        assert_eq!(
            conn.process_new_packets().unwrap_err(),
            PeerMisbehaved::MissingPskExtensionInSecondClientHello.into(),
        );
    }
