#[test]
fn test_ecdsa_prime256v1_sha512_using_rsa_algorithm() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "ecdsa-prime256v1-sha512-using-rsa-algorithm.pem"
        ),),
        Err(unsupported_for_rsa(
            &alg_id::RSA_PKCS1_SHA512,
            &alg_id::ECDSA_P256,
        ))
    );
}
