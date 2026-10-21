#[test]
    fn test_client_requiring_rpk_rejects_server_that_only_demands_x509_by_omission() {
        assert_eq!(
            client_requiring_rpk_receives_server_ee(ServerExtensions {
                server_certificate_type: Some(CertificateType::RawPublicKey),
                ..ServerExtensions::default()
            }),
            Err(PeerIncompatible::IncorrectCertificateTypeExtension.into())
        );
    }
