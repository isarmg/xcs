#[test]
fn test_ecdsa_prime256v1_sha512_using_ecmqv_key() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "ecdsa-prime256v1-sha512-using-ecmqv-key.pem"
        )),
        Err(unsupported_for_ecdsa(
            &alg_id::ECDSA_SHA512,
            include_bytes!("../tests/signatures/alg-ecmqv-secp256r1.der")
        ))
    );
}
