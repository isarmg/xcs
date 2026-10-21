#[test]
fn test_rsa_pkcs1_sha1_wrong_algorithm() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("rsa-pkcs1-sha1-wrong-algorithm.pem")),
        Err(invalid_rsa_signature())
    );
}
