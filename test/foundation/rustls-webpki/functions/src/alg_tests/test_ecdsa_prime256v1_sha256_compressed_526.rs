#[test]
fn test_ecdsa_prime256v1_sha256_compressed() {
    assert_eq!(
        test_verify_signed_data(test_file_bytes!(
            "ours/ecdsa-prime256v1-sha256-compressed.pem"
        )),
        OK_IF_POINT_COMPRESSION_SUPPORTED
    );
}
