#[test]
fn test_rsa_pss_sha512_salt64_corrupted_data() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "ours/rsa-pss-sha512-salt64-corrupted-data.pem"
        )),
        Err(invalid_rsa_signature())
    );
}
