#[test]
fn test_rsa_pss_sha384_salt48_corrupted_data() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "ours/rsa-pss-sha384-salt48-corrupted-data.pem"
        )),
        Err(invalid_rsa_signature())
    );
}
