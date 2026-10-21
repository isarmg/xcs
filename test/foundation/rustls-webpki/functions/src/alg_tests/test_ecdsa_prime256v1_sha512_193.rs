#[test]
fn test_ecdsa_prime256v1_sha512() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("ecdsa-prime256v1-sha512.pem")),
        Err(unsupported_for_ecdsa(
            &alg_id::ECDSA_SHA512,
            &alg_id::ECDSA_P256,
        ))
    );
}
