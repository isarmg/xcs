#[test]
fn test_rsa_pkcs1_sha256() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("rsa-pkcs1-sha256.pem")),
        Err(invalid_rsa_signature())
    );
}
