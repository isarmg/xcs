#[test]
fn test_rsa_pss_sha512_salt64() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("ours/rsa-pss-sha512-salt64.pem")),
        maybe_rsa()
    );
}
