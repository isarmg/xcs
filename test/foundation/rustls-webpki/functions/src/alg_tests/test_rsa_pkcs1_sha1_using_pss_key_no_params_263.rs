#[test]
fn test_rsa_pkcs1_sha1_using_pss_key_no_params() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "rsa-pkcs1-sha1-using-pss-key-no-params.pem"
        )),
        Err(unsupported(include_bytes!(
            "../tests/signatures/alg-rsae-sha1.der"
        )))
    );
}
