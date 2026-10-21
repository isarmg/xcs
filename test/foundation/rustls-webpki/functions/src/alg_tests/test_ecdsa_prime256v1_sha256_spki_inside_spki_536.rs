#[test]
fn test_ecdsa_prime256v1_sha256_spki_inside_spki() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "ours/ecdsa-prime256v1-sha256-spki-inside-spki.pem"
        )),
        Err(Error::InvalidSignatureForPublicKey)
    );
}
