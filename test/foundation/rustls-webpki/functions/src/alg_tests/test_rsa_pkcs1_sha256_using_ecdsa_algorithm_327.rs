#[test]
fn test_rsa_pkcs1_sha256_using_ecdsa_algorithm() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "rsa-pkcs1-sha256-using-ecdsa-algorithm.pem"
        )),
        Err(Error::UnsupportedSignatureAlgorithmForPublicKeyContext(
            UnsupportedSignatureAlgorithmForPublicKeyContext {
                #[cfg(feature = "alloc")]
                signature_algorithm_id: alg_id::ECDSA_SHA256.as_ref().to_vec(),
                #[cfg(feature = "alloc")]
                public_key_algorithm_id: alg_id::RSA_ENCRYPTION.as_ref().to_vec(),
            }
        ))
    );
}
