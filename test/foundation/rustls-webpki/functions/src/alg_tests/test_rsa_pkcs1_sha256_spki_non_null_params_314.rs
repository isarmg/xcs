#[test]
fn test_rsa_pkcs1_sha256_spki_non_null_params() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "rsa-pkcs1-sha256-spki-non-null-params.pem"
        )),
        Err(unsupported_for_rsa(
            &alg_id::RSA_PKCS1_SHA256,
            include_bytes!("../tests/signatures/alg-rsae-bad-params.der")
        ))
    );
}
