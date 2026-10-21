#[test]
fn test_rsa_pkcs1_sha1_bad_key_der_length() {
    assert_eq!(
        test_parse_spki_bad_outer(test_file_bytes!("rsa-pkcs1-sha1-bad-key-der-length.pem")),
        Err(Error::BadDer)
    );
}
