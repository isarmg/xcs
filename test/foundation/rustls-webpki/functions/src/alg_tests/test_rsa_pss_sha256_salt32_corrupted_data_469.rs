#[test]
fn test_rsa_pss_sha256_salt32_corrupted_data() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "ours/rsa-pss-sha256-salt32-corrupted-data.pem"
        )),
        Err(invalid_rsa_signature())
    );
}
