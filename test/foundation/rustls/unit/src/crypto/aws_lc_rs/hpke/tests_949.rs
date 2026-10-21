#[cfg(test)]
mod tests {
    use alloc::{format, vec};

    use super::*;

    #[test]
    fn smoke_test() {
        for suite in ALL_SUPPORTED_SUITES {
            _ = format!("{suite:?}"); // HpkeAwsLcRs suites should be Debug.

            // We should be able to generate a random keypair.
            let (pk, sk) = suite.generate_key_pair().unwrap();

            // Info value corresponds to the first RFC 9180 base mode test vector.
            let info = &[
                0x4f, 0x64, 0x65, 0x20, 0x6f, 0x6e, 0x20, 0x61, 0x20, 0x47, 0x72, 0x65, 0x63, 0x69,
                0x61, 0x6e, 0x20, 0x55, 0x72, 0x6e,
            ][..];

            // We should be able to set up a sealer.
            let (enc, mut sealer) = suite.setup_sealer(info, &pk).unwrap();

            _ = format!("{sealer:?}"); // Sealer should be Debug.

            // Setting up a sealer with an invalid public key should fail.
            let bad_setup_res = suite.setup_sealer(info, &HpkePublicKey(vec![]));
            assert!(matches!(bad_setup_res.unwrap_err(), Error::Other(_)));

            // We should be able to seal some plaintext.
            let aad = &[0xC0, 0xFF, 0xEE];
            let pt = &[0xF0, 0x0D];
            let ct = sealer.seal(aad, pt).unwrap();

            // We should be able to set up an opener.
            let mut opener = suite
                .setup_opener(&enc, info, &sk)
                .unwrap();
            _ = format!("{opener:?}"); // Opener should be Debug.

            // Setting up an opener with an invalid private key should fail.
            let bad_key_res = suite.setup_opener(&enc, info, &HpkePrivateKey::from(vec![]));
            assert!(matches!(bad_key_res.unwrap_err(), Error::Other(_)));

            // Opening the plaintext should work with the correct opener and aad.
            let pt_prime = opener.open(aad, &ct).unwrap();
            assert_eq!(pt_prime, pt);

            // Opening the plaintext with the correct opener and wrong aad should fail.
            let open_res = opener.open(&[0x0], &ct);
            assert!(matches!(open_res.unwrap_err(), Error::Other(_)));

            // Opening the plaintext with the wrong opener should fail.
            let mut sk_rm_prime = sk.secret_bytes().to_vec();
            sk_rm_prime[10] ^= 0xFF; // Corrupt a byte of the private key.
            let mut opener_two = suite
                .setup_opener(&enc, info, &HpkePrivateKey::from(sk_rm_prime))
                .unwrap();
            let open_res = opener_two.open(aad, &ct);
            assert!(matches!(open_res.unwrap_err(), Error::Other(_)));
        }
    }

    #[cfg(not(feature = "fips"))] // Ensure all supported suites are available to test.
    #[test]
    fn test_fips() {
        let testcases: &[(&dyn Hpke, bool)] = &[
            // FIPS compatible.
            (DH_KEM_P256_HKDF_SHA256_AES_128, true),
            (DH_KEM_P256_HKDF_SHA256_AES_256, true),
            (DH_KEM_P384_HKDF_SHA384_AES_128, true),
            (DH_KEM_P384_HKDF_SHA384_AES_256, true),
            (DH_KEM_P521_HKDF_SHA512_AES_128, true),
            (DH_KEM_P521_HKDF_SHA512_AES_256, true),
            // AEAD is not FIPS compatible.
            (DH_KEM_P256_HKDF_SHA256_CHACHA20_POLY1305, false),
            (DH_KEM_P384_HKDF_SHA384_CHACHA20_POLY1305, false),
            (DH_KEM_P521_HKDF_SHA512_CHACHA20_POLY1305, false),
            // KEM is not FIPS compatible.
            (DH_KEM_X25519_HKDF_SHA256_AES_128, false),
            (DH_KEM_X25519_HKDF_SHA256_AES_256, false),
            (DH_KEM_X25519_HKDF_SHA256_CHACHA20_POLY1305, false),
        ];
        for (suite, expected) in testcases {
            assert_eq!(suite.fips(), *expected);
        }
    }

    #[test]
    fn failed_open_does_not_consume_sequence() {
        for suite in ALL_SUPPORTED_SUITES {
            let (pk, sk) = suite.generate_key_pair().unwrap();
            let (enc, mut sealer) = suite
                .setup_sealer(b"example", &pk)
                .unwrap();
            let ct0 = sealer
                .seal(b"aad", b"message 0")
                .unwrap();
            let ct1 = sealer
                .seal(b"aad", b"message 1")
                .unwrap();

            let mut opener = suite
                .setup_opener(&enc, b"example", &sk)
                .unwrap();
            assert!(opener.open(b"wrong aad", &ct0).is_err());
            assert_eq!(opener.open(b"aad", &ct0).unwrap(), b"message 0");
            assert_eq!(opener.open(b"aad", &ct1).unwrap(), b"message 1");
        }
    }
}
