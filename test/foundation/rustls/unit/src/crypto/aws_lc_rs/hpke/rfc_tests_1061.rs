#[cfg(test)]
mod rfc_tests {
    use alloc::string::String;
    use std::fs::File;
    use std::println;

    use data_serialization::Deserialize;

    use super::*;

    /// Confirm open/seal operations work using the test vectors from [RFC 9180 Appendix A].
    ///
    /// [RFC 9180 Appendix A]: https://www.rfc-editor.org/rfc/rfc9180#TestVectors
    #[test]
    fn check_test_vectors() {
        for (idx, vec) in test_vectors().into_iter().enumerate() {
            let Some(hpke) = vec.applicable() else {
                println!("skipping inapplicable vector {idx}");
                continue;
            };

            println!("testing vector {idx}");
            let pk_r = HpkePublicKey(hex::decode(vec.pk_rm).unwrap());
            let sk_r = HpkePrivateKey::from(hex::decode(vec.sk_rm).unwrap());
            let sk_em = hex::decode(vec.sk_em).unwrap();
            let info = hex::decode(vec.info).unwrap();
            let expected_enc = hex::decode(vec.enc).unwrap();

            let (enc, mut sealer) = hpke
                .setup_test_sealer(&info, &pk_r, &sk_em)
                .unwrap();
            assert_eq!(enc.0, expected_enc);

            let mut opener = hpke
                .setup_opener(&enc, &info, &sk_r)
                .unwrap();

            for test_encryption in vec.encryptions {
                let aad = hex::decode(test_encryption.aad).unwrap();
                let pt = hex::decode(test_encryption.pt).unwrap();
                let expected_ct = hex::decode(test_encryption.ct).unwrap();

                let ciphertext = sealer.seal(&aad, &pt).unwrap();
                assert_eq!(ciphertext, expected_ct);

                let plaintext = opener.open(&aad, &ciphertext).unwrap();
                assert_eq!(plaintext, pt);
            }
        }
    }

    trait TestHpke: Hpke {
        fn setup_test_sealer(
            &self,
            info: &[u8],
            pub_key: &HpkePublicKey,
            sk_em: &[u8],
        ) -> Result<(EncapsulatedSecret, Box<dyn HpkeSealer + 'static>), Error>;
    }

    impl<const KEY_SIZE: usize, const KDF_SIZE: usize> TestHpke for HpkeAwsLcRs<KEY_SIZE, KDF_SIZE> {
        fn setup_test_sealer(
            &self,
            info: &[u8],
            pub_key: &HpkePublicKey,
            sk_em: &[u8],
        ) -> Result<(EncapsulatedSecret, Box<dyn HpkeSealer + 'static>), Error> {
            let (encap, sealer) = Sealer::test_only_new(self, info, pub_key, sk_em)?;
            Ok((encap, Box::new(sealer)))
        }
    }

    static TEST_SUITES: &[&dyn TestHpke] = &[
        DH_KEM_P256_HKDF_SHA256_AES_128,
        DH_KEM_P256_HKDF_SHA256_AES_256,
        #[cfg(not(feature = "fips"))]
        DH_KEM_P256_HKDF_SHA256_CHACHA20_POLY1305,
        DH_KEM_P384_HKDF_SHA384_AES_128,
        DH_KEM_P384_HKDF_SHA384_AES_256,
        #[cfg(not(feature = "fips"))]
        DH_KEM_P384_HKDF_SHA384_CHACHA20_POLY1305,
        DH_KEM_P521_HKDF_SHA512_AES_128,
        DH_KEM_P521_HKDF_SHA512_AES_256,
        #[cfg(not(feature = "fips"))]
        DH_KEM_P521_HKDF_SHA512_CHACHA20_POLY1305,
        #[cfg(not(feature = "fips"))]
        DH_KEM_X25519_HKDF_SHA256_AES_128,
        #[cfg(not(feature = "fips"))]
        DH_KEM_X25519_HKDF_SHA256_AES_256,
        #[cfg(not(feature = "fips"))]
        DH_KEM_X25519_HKDF_SHA256_CHACHA20_POLY1305,
    ];

    #[derive(Deserialize, Debug)]
    struct TestVector {
        mode: u8,
        kem_id: u16,
        kdf_id: u16,
        aead_id: u16,
        info: String,
        #[serialization(rename(deserialize = "pkRm"))]
        pk_rm: String,
        #[serialization(rename(deserialize = "skRm"))]
        sk_rm: String,
        #[serialization(rename(deserialize = "skEm"))]
        sk_em: String,
        enc: String,
        encryptions: Vec<TestEncryption>,
    }

    #[derive(Deserialize, Debug)]
    struct TestEncryption {
        aad: String,
        pt: String,
        ct: String,
    }

    impl TestVector {
        fn suite(&self) -> HpkeSuite {
            HpkeSuite {
                kem: HpkeKem::from(self.kem_id),
                sym: HpkeSymmetricCipherSuite {
                    kdf_id: HpkeKdf::from(self.kdf_id),
                    aead_id: HpkeAead::from(self.aead_id),
                },
            }
        }

        fn applicable(&self) -> Option<&'static dyn TestHpke> {
            // Only base mode test vectors for supported suites are applicable.
            if self.mode != 0 {
                return None;
            }

            Self::lookup_suite(self.suite(), TEST_SUITES)
        }

        fn lookup_suite(
            suite: HpkeSuite,
            supported: &[&'static dyn TestHpke],
        ) -> Option<&'static dyn TestHpke> {
            supported
                .iter()
                .find(|s| s.suite() == suite)
                .copied()
        }
    }

    fn test_vectors() -> Vec<TestVector> {
        json_codec::from_reader(
            &mut File::open("../rustls-provider-test/tests/rfc-9180-test-vectors.json")
                .expect("failed to open test vectors data file"),
        )
        .expect("failed to deserialize test vectors")
    }
}
