#[test]
fn test_ecdsa_prime256v1_sha512_wrong_signature_format() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "ecdsa-prime256v1-sha512-wrong-signature-format.pem"
        )),
        Err(unsupported_for_ecdsa(
            &alg_id::ECDSA_SHA512,
            &alg_id::ECDSA_P256,
        ))
    );
}
