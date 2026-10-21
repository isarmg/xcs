#[test]
fn test_rsa_pss_sha256_salt10_using_pss_key_with_params() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "rsa-pss-sha256-salt10-using-pss-key-with-params.pem"
        )),
        Err(unsupported(include_bytes!(
            "../tests/signatures/alg-rsapss-sha256-mgf1-sha256-salt10.der"
        )))
    );
}
