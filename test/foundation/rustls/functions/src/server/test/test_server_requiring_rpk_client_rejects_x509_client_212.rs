#[test]
    fn test_server_requiring_rpk_client_rejects_x509_client() {
        let mut ch = minimal_client_hello();
        ch.extensions.client_certificate_types = Some(vec![CertificateType::X509]);
        let ch = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::handshake(HandshakeMessagePayload(
                HandshakePayload::ClientHello(ch),
            )),
        };

        let mut conn = ServerConnection::new(server_config_for_rpk().into()).unwrap();
        conn.read_tls(&mut ch.into_wire_bytes().as_slice())
            .unwrap();
        assert_eq!(
            conn.process_new_packets().unwrap_err(),
            PeerIncompatible::IncorrectCertificateTypeExtension.into(),
        );
    }
