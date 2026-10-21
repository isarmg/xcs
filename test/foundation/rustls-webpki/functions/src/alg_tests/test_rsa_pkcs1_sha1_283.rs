#[test]
fn test_rsa_pkcs1_sha1() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!("rsa-pkcs1-sha1.pem")),
        Err(unsupported(include_bytes!(
            "../tests/signatures/alg-rsae-sha1.der"
        )))
    );
}
