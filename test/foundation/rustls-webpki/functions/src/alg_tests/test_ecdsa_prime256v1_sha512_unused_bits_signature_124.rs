#[test]
fn test_ecdsa_prime256v1_sha512_unused_bits_signature() {
    assert_eq!(
        test_verify_signed_data_signature_outer(test_file_bytes!(
            "ecdsa-prime256v1-sha512-unused-bits-signature.pem"
        )),
        Err(Error::BadDer)
    );
}
