#[test]
fn test_rsa_pkcs1_sha256_using_id_ea_rsa() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("rsa-pkcs1-sha256-using-id-ea-rsa.pem")),
        Err(unsupported_for_rsa(
            &alg_id::RSA_PKCS1_SHA256,
            include_bytes!("../tests/signatures/alg-rsa-null-params.der")
        ))
    );
}
