#[test]
fn test_rsa_pkcs1_sha1_key_params_absent() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("rsa-pkcs1-sha1-key-params-absent.pem")),
        Err(unsupported(include_bytes!(
            "../tests/signatures/alg-rsae-sha1.der"
        )))
    );
}
