#[test]
    fn self_signed_ml_dsa_44() {
        let pem = include_bytes!("../tests/tls_server_certs/ml-dsa-44-certificate.pem");
        let der = CertificateDer::from_pem_slice(pem).unwrap();
        let cert = Cert::from_der(Input::from(&der)).unwrap();
        verify_signed_data(
            SUPPORTED_ALGORITHMS_IN_TESTS,
            cert.spki,
            &cert.signed_data,
            &mut Budget::default(),
        )
        .unwrap();
    }
