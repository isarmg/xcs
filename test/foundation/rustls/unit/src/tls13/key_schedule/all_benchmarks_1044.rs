#[cfg(all(test, bench))]
#[macro_rules_attribute::apply(bench_for_each_provider)]
mod benchmarks {
    #[bench]
    fn bench_sha256(b: &mut test::Bencher) {
        use core::fmt::Debug;

        use super::provider::tls13::TLS13_CHACHA20_POLY1305_SHA256_INTERNAL;
        use super::{KeySchedule, SecretKind, derive_traffic_iv, derive_traffic_key};
        use crate::KeyLog;

        fn extract_traffic_secret(ks: &KeySchedule, kind: SecretKind) {
            #[derive(Debug)]
            struct Log;

            impl KeyLog for Log {
                fn log(&self, _label: &str, _client_random: &[u8], _secret: &[u8]) {}
            }

            let hash = [0u8; 32];
            let traffic_secret = ks.derive_logged_secret(kind, &hash, &Log, &[0u8; 32]);
            let traffic_secret_expander = TLS13_CHACHA20_POLY1305_SHA256_INTERNAL
                .hkdf_provider
                .expander_for_okm(&traffic_secret);
            test::black_box(derive_traffic_key(
                traffic_secret_expander.as_ref(),
                TLS13_CHACHA20_POLY1305_SHA256_INTERNAL.aead_alg,
            ));
            test::black_box(derive_traffic_iv(traffic_secret_expander.as_ref()));
        }

        b.iter(|| {
            let mut ks =
                KeySchedule::new_with_empty_secret(TLS13_CHACHA20_POLY1305_SHA256_INTERNAL);
            ks.input_secret(&[0u8; 32]);

            extract_traffic_secret(&ks, SecretKind::ClientHandshakeTrafficSecret);
            extract_traffic_secret(&ks, SecretKind::ServerHandshakeTrafficSecret);

            ks.input_empty();

            extract_traffic_secret(&ks, SecretKind::ClientApplicationTrafficSecret);
            extract_traffic_secret(&ks, SecretKind::ServerApplicationTrafficSecret);
        });
    }
}
