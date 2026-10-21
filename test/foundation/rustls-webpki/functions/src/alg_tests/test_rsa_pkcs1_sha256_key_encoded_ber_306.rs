#[test]
fn test_rsa_pkcs1_sha256_key_encoded_ber() {
    assert_eq!(
        test_parse_spki_bad_outer(test_file_bytes!("rsa-pkcs1-sha256-key-encoded-ber.pem")),
        Err(Error::BadDer)
    );
}
