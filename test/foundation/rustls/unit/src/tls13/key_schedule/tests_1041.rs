#[cfg(test)]
#[macro_rules_attribute::apply(test_for_each_provider)]
mod tests {
    use core::fmt::Debug;
    use std::prelude::v1::*;
    use std::vec;

    use super::provider::ring_like::aead;
    use super::provider::tls13::{
        TLS13_AES_128_GCM_SHA256_INTERNAL, TLS13_CHACHA20_POLY1305_SHA256_INTERNAL,
    };
    use super::{KeySchedule, SecretKind, derive_traffic_iv, derive_traffic_key};
    use crate::KeyLog;
    use crate::msgs::enums::HashAlgorithm;

    #[test]
    fn empty_hash() {
        let sha256 = super::provider::tls13::TLS13_AES_128_GCM_SHA256
            .tls13()
            .unwrap()
            .common
            .hash_provider;
        let sha384 = super::provider::tls13::TLS13_AES_256_GCM_SHA384
            .tls13()
            .unwrap()
            .common
            .hash_provider;

        assert!(
            sha256.start().finish().as_ref()
                == HashAlgorithm::SHA256
                    .hash_for_empty_input()
                    .unwrap()
                    .as_ref()
        );
        assert!(
            sha384.start().finish().as_ref()
                == HashAlgorithm::SHA384
                    .hash_for_empty_input()
                    .unwrap()
                    .as_ref()
        );

        // a theoretical example of unsupported hash
        assert!(
            HashAlgorithm::SHA1
                .hash_for_empty_input()
                .is_none()
        );
    }

    #[test]
    fn test_vectors() {
        /* These test vectors generated with OpenSSL. */
        let hs_start_hash = [
            0xec, 0x14, 0x7a, 0x06, 0xde, 0xa3, 0xc8, 0x84, 0x6c, 0x02, 0xb2, 0x23, 0x8e, 0x41,
            0xbd, 0xdc, 0x9d, 0x89, 0xf9, 0xae, 0xa1, 0x7b, 0x5e, 0xfd, 0x4d, 0x74, 0x82, 0xaf,
            0x75, 0x88, 0x1c, 0x0a,
        ];

        let hs_full_hash = [
            0x75, 0x1a, 0x3d, 0x4a, 0x14, 0xdf, 0xab, 0xeb, 0x68, 0xe9, 0x2c, 0xa5, 0x91, 0x8e,
            0x24, 0x08, 0xb9, 0xbc, 0xb0, 0x74, 0x89, 0x82, 0xec, 0x9c, 0x32, 0x30, 0xac, 0x30,
            0xbb, 0xeb, 0x23, 0xe2,
        ];

        let ecdhe_secret = [
            0xe7, 0xb8, 0xfe, 0xf8, 0x90, 0x3b, 0x52, 0x0c, 0xb9, 0xa1, 0x89, 0x71, 0xb6, 0x9d,
            0xd4, 0x5d, 0xca, 0x53, 0xce, 0x2f, 0x12, 0xbf, 0x3b, 0xef, 0x93, 0x15, 0xe3, 0x12,
            0x71, 0xdf, 0x4b, 0x40,
        ];

        let client_hts = [
            0x61, 0x7b, 0x35, 0x07, 0x6b, 0x9d, 0x0e, 0x08, 0xcf, 0x73, 0x1d, 0x94, 0xa8, 0x66,
            0x14, 0x78, 0x41, 0x09, 0xef, 0x25, 0x55, 0x51, 0x92, 0x1d, 0xd4, 0x6e, 0x04, 0x01,
            0x35, 0xcf, 0x46, 0xab,
        ];

        let client_hts_key = [
            0x62, 0xd0, 0xdd, 0x00, 0xf6, 0x96, 0x19, 0xd3, 0xb8, 0x19, 0x3a, 0xb4, 0xa0, 0x95,
            0x85, 0xa7,
        ];

        let client_hts_iv = [
            0xff, 0xf7, 0x5d, 0xf5, 0xad, 0x35, 0xd5, 0xcb, 0x3c, 0x53, 0xf3, 0xa9,
        ];

        let server_hts = [
            0xfc, 0xf7, 0xdf, 0xe6, 0x4f, 0xa2, 0xc0, 0x4f, 0x62, 0x35, 0x38, 0x7f, 0x43, 0x4e,
            0x01, 0x42, 0x23, 0x36, 0xd9, 0xc0, 0x39, 0xde, 0x68, 0x47, 0xa0, 0xb9, 0xdd, 0xcf,
            0x29, 0xa8, 0x87, 0x59,
        ];

        let server_hts_key = [
            0x04, 0x67, 0xf3, 0x16, 0xa8, 0x05, 0xb8, 0xc4, 0x97, 0xee, 0x67, 0x04, 0x7b, 0xbc,
            0xbc, 0x54,
        ];

        let server_hts_iv = [
            0xde, 0x83, 0xa7, 0x3e, 0x9d, 0x81, 0x4b, 0x04, 0xc4, 0x8b, 0x78, 0x09,
        ];

        let client_ats = [
            0xc1, 0x4a, 0x6d, 0x79, 0x76, 0xd8, 0x10, 0x2b, 0x5a, 0x0c, 0x99, 0x51, 0x49, 0x3f,
            0xee, 0x87, 0xdc, 0xaf, 0xf8, 0x2c, 0x24, 0xca, 0xb2, 0x14, 0xe8, 0xbe, 0x71, 0xa8,
            0x20, 0x6d, 0xbd, 0xa5,
        ];

        let client_ats_key = [
            0xcc, 0x9f, 0x5f, 0x98, 0x0b, 0x5f, 0x10, 0x30, 0x6c, 0xba, 0xd7, 0xbe, 0x98, 0xd7,
            0x57, 0x2e,
        ];

        let client_ats_iv = [
            0xb8, 0x09, 0x29, 0xe8, 0xd0, 0x2c, 0x70, 0xf6, 0x11, 0x62, 0xed, 0x6b,
        ];

        let server_ats = [
            0x2c, 0x90, 0x77, 0x38, 0xd3, 0xf8, 0x37, 0x02, 0xd1, 0xe4, 0x59, 0x8f, 0x48, 0x48,
            0x53, 0x1d, 0x9f, 0x93, 0x65, 0x49, 0x1b, 0x9f, 0x7f, 0x52, 0xc8, 0x22, 0x29, 0x0d,
            0x4c, 0x23, 0x21, 0x92,
        ];

        let server_ats_key = [
            0x0c, 0xb2, 0x95, 0x62, 0xd8, 0xd8, 0x8f, 0x48, 0xb0, 0x2c, 0xbf, 0xbe, 0xd7, 0xe6,
            0x2b, 0xb3,
        ];

        let server_ats_iv = [
            0x0d, 0xb2, 0x8f, 0x98, 0x85, 0x86, 0xa1, 0xb7, 0xe4, 0xd5, 0xc6, 0x9c,
        ];

        let mut ks = KeySchedule::new_with_empty_secret(TLS13_CHACHA20_POLY1305_SHA256_INTERNAL);
        ks.input_secret(&ecdhe_secret);

        assert_traffic_secret(
            &ks,
            SecretKind::ClientHandshakeTrafficSecret,
            &hs_start_hash,
            &client_hts,
            &client_hts_key,
            &client_hts_iv,
        );

        assert_traffic_secret(
            &ks,
            SecretKind::ServerHandshakeTrafficSecret,
            &hs_start_hash,
            &server_hts,
            &server_hts_key,
            &server_hts_iv,
        );

        ks.input_empty();

        assert_traffic_secret(
            &ks,
            SecretKind::ClientApplicationTrafficSecret,
            &hs_full_hash,
            &client_ats,
            &client_ats_key,
            &client_ats_iv,
        );

        assert_traffic_secret(
            &ks,
            SecretKind::ServerApplicationTrafficSecret,
            &hs_full_hash,
            &server_ats,
            &server_ats_key,
            &server_ats_iv,
        );
    }

    fn assert_traffic_secret(
        ks: &KeySchedule,
        kind: SecretKind,
        hash: &[u8],
        expected_traffic_secret: &[u8],
        expected_key: &[u8],
        expected_iv: &[u8],
    ) {
        #[derive(Debug)]
        struct Log<'a>(&'a [u8]);
        impl KeyLog for Log<'_> {
            fn log(&self, _label: &str, _client_random: &[u8], secret: &[u8]) {
                assert_eq!(self.0, secret);
            }
        }
        let log = Log(expected_traffic_secret);
        let traffic_secret = ks.derive_logged_secret(kind, hash, &log, &[0; 32]);

        // Since we can't test key equality, we test the output of sealing with the key instead.
        let aead_alg = &aead::AES_128_GCM;
        let expander = TLS13_AES_128_GCM_SHA256_INTERNAL
            .hkdf_provider
            .expander_for_okm(&traffic_secret);
        let key = derive_traffic_key(
            expander.as_ref(),
            TLS13_AES_128_GCM_SHA256_INTERNAL.aead_alg,
        );
        let key = aead::UnboundKey::new(aead_alg, key.as_ref()).unwrap();
        let seal_output = seal_zeroes(key);
        let expected_key = aead::UnboundKey::new(aead_alg, expected_key).unwrap();
        let expected_seal_output = seal_zeroes(expected_key);
        assert_eq!(seal_output, expected_seal_output);
        assert!(seal_output.len() >= 48); // Sanity check.

        let iv = derive_traffic_iv(expander.as_ref());
        assert_eq!(iv.as_ref(), expected_iv);
    }

    fn seal_zeroes(key: aead::UnboundKey) -> Vec<u8> {
        let key = aead::LessSafeKey::new(key);
        let mut seal_output = vec![0; 32];
        key.seal_in_place_append_tag(
            aead::Nonce::assume_unique_for_key([0; aead::NONCE_LEN]),
            aead::Aad::empty(),
            &mut seal_output,
        )
        .unwrap();
        seal_output
    }
}
