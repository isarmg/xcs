#[test]
fn test_rsa_pss_sha256_mgf1_sha512_salt33() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("rsa-pss-sha256-mgf1-sha512-salt33.pem")),
        Err(unsupported(include_bytes!(
            "../tests/signatures/alg-rsapss-sha256-mgf1-sha512-salt33.der"
        )))
    );
}
