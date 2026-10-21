#[test]
fn test_rsa_using_ec_key() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("rsa-using-ec-key.pem")),
        Err(unsupported_for_rsa(
            &alg_id::RSA_PKCS1_SHA256,
            &alg_id::ECDSA_P256
        ))
    );
}
