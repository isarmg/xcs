#[test]
fn test_rsa_pss_sha1_wrong_salt() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("rsa-pss-sha1-wrong-salt.pem")),
        Err(unsupported(include_bytes!(
            "../tests/signatures/alg-rsapss-salt23.der"
        )))
    );
}
