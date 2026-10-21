#[test]
fn test_ecdsa_secp384r1_sha256_corrupted_data() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "ecdsa-secp384r1-sha256-corrupted-data.pem"
        )),
        Err(Error::InvalidSignatureForPublicKey)
    );
}
