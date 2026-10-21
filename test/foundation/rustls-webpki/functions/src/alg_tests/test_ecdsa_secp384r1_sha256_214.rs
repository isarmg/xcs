#[test]
fn test_ecdsa_secp384r1_sha256() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("ecdsa-secp384r1-sha256.pem")),
        Ok(())
    );
}
