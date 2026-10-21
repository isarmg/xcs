#[test]
fn test_ecdsa_prime256v1_sha512_spki_params_null() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "ecdsa-prime256v1-sha512-spki-params-null.pem"
        )),
        Err(unsupported_for_ecdsa(
            &alg_id::ECDSA_SHA512,
            include_bytes!("../tests/signatures/alg-id-ecpublickey-params-null.der")
        ))
    );
}
