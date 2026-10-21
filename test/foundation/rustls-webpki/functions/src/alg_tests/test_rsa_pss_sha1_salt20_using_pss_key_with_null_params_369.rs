#[test]
fn test_rsa_pss_sha1_salt20_using_pss_key_with_null_params() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "rsa-pss-sha1-salt20-using-pss-key-with-null-params.pem"
        )),
        Err(unsupported(include_bytes!(
            "../tests/signatures/alg-rsapss-defaults.der"
        )))
    );
}
