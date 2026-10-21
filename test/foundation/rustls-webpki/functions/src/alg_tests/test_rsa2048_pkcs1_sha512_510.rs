#[test]
fn test_rsa2048_pkcs1_sha512() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("rsa2048-pkcs1-sha512.pem")),
        maybe_rsa()
    );
}
