#[cfg(test)]
#[macro_rules_attribute::apply(test_for_each_provider)]
mod tests {
    use core::time::Duration;
    use std::prelude::v1::*;
    use std::{format, vec};

    use pki_types::{ServerName, UnixTime};

    #[cfg(feature = "std")]
    use super::ClientSessionMemoryCache;
    use super::NoClientSessionStorage;
    use super::provider::cipher_suite;
    use crate::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
    use crate::client::{ClientSessionStore, ResolvesClientCert};
    use crate::msgs::base::PayloadU16;
    use crate::msgs::enums::NamedGroup;
    use crate::msgs::handshake::CertificateChain;
    #[cfg(feature = "tls12")]
    use crate::msgs::handshake::SessionId;
    use crate::msgs::persist::Tls13ClientSessionValue;
    use crate::pki_types::CertificateDer;
    use crate::suites::SupportedCipherSuite;
    use crate::sync::Arc;
    use crate::{DigitallySignedStruct, Error, SignatureScheme, sign};

    #[test]
    fn test_noclientsessionstorage_does_nothing() {
        let c = NoClientSessionStorage {};
        let name = ServerName::try_from("example.com").unwrap();
        let now = UnixTime::now();
        let server_cert_verifier: Arc<dyn ServerCertVerifier> = Arc::new(DummyServerCertVerifier);
        let resolves_client_cert: Arc<dyn ResolvesClientCert> = Arc::new(DummyResolvesClientCert);

        c.set_kx_hint(name.clone(), NamedGroup::X25519);
        assert_eq!(None, c.kx_hint(&name));

        #[cfg(feature = "tls12")]
        {
            use crate::msgs::persist::Tls12ClientSessionValue;
            let SupportedCipherSuite::Tls12(tls12_suite) =
                cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
            else {
                unreachable!()
            };

            c.set_tls12_session(
                name.clone(),
                Tls12ClientSessionValue::new(
                    tls12_suite,
                    SessionId::empty(),
                    Arc::new(PayloadU16::empty()),
                    &[],
                    CertificateChain::default(),
                    &server_cert_verifier,
                    &resolves_client_cert,
                    now,
                    0,
                    true,
                ),
            );
            assert!(c.tls12_session(&name).is_none());
            c.remove_tls12_session(&name);
        }

        let SupportedCipherSuite::Tls13(tls13_suite) = cipher_suite::TLS13_AES_256_GCM_SHA384
        else {
            unreachable!();
        };
        c.insert_tls13_ticket(
            name.clone(),
            Tls13ClientSessionValue::new(
                tls13_suite,
                Arc::new(PayloadU16::empty()),
                &[],
                CertificateChain::default(),
                &server_cert_verifier,
                &resolves_client_cert,
                now,
                0,
                0,
                0,
            ),
        );
        assert!(c.take_tls13_ticket(&name).is_none());
    }

    #[test]
    fn debug_of_session_types() {
        let server_cert_verifier: Arc<dyn ServerCertVerifier> = Arc::new(DummyServerCertVerifier);
        let resolves_client_cert: Arc<dyn ResolvesClientCert> = Arc::new(DummyResolvesClientCert);

        #[cfg(feature = "tls12")]
        {
            use crate::msgs::persist::Tls12ClientSessionValue;
            let SupportedCipherSuite::Tls12(tls12_suite) =
                cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
            else {
                unreachable!()
            };

            let tls12 = Tls12ClientSessionValue::new(
                tls12_suite,
                SessionId::empty(),
                Arc::new(PayloadU16::new(vec![5])),
                &[0xa5; 48],
                CertificateChain::default(),
                &server_cert_verifier,
                &resolves_client_cert,
                UnixTime::since_unix_epoch(Duration::from_secs(1)),
                2,
                true,
            );
            let debug = format!("{tls12:?}");
            assert!(
                debug.contains("ClientSessionCommon { ticket: 05, epoch: 1, lifetime_secs: 2,")
            );
            assert!(!debug.contains("a5a5"), "{debug}");
        }

        let SupportedCipherSuite::Tls13(tls13_suite) = cipher_suite::TLS13_AES_256_GCM_SHA384
        else {
            unreachable!();
        };
        let tls13 = Tls13ClientSessionValue::new(
            tls13_suite,
            Arc::new(PayloadU16::new(vec![5])),
            &[0xa5; 32],
            CertificateChain::default(),
            &server_cert_verifier,
            &resolves_client_cert,
            UnixTime::since_unix_epoch(Duration::from_secs(1)),
            2,
            3,
            0,
        );
        let debug = format!("{tls13:?}");
        assert!(debug.contains("ClientSessionCommon { ticket: 05, epoch: 1, lifetime_secs: 2,"));
        assert!(!debug.contains("a5a5"), "{debug}");
    }

    /// For each `size` from 1 to 8, the cache keeps one server's key
    /// exchange hint and TLS 1.3 ticket.
    #[cfg(feature = "std")]
    #[test]
    fn test_clientsessionmemorycache_small_sizes_retain_server() {
        for size in 1..=8 {
            let c = ClientSessionMemoryCache::new(size);
            let name = ServerName::try_from("example.com").unwrap();

            c.set_kx_hint(name.clone(), NamedGroup::X25519);
            assert_eq!(c.kx_hint(&name), Some(NamedGroup::X25519), "size {size}");

            c.insert_tls13_ticket(name.clone(), tls13_session(UnixTime::now()));
            assert!(c.take_tls13_ticket(&name).is_some(), "size {size}");
        }
    }

    /// A cache of size 9 remembers two servers: after three servers are
    /// added, the first is evicted and the other two remain.
    #[cfg(feature = "std")]
    #[test]
    fn test_clientsessionmemorycache_evicts_oldest_server() {
        let c = ClientSessionMemoryCache::new(9);
        let first = ServerName::try_from("first.example.com").unwrap();
        let second = ServerName::try_from("second.example.com").unwrap();
        let third = ServerName::try_from("third.example.com").unwrap();

        c.set_kx_hint(first.clone(), NamedGroup::X25519);
        c.set_kx_hint(second.clone(), NamedGroup::X25519);
        c.set_kx_hint(third.clone(), NamedGroup::X25519);

        assert_eq!(c.kx_hint(&first), None);
        assert_eq!(c.kx_hint(&second), Some(NamedGroup::X25519));
        assert_eq!(c.kx_hint(&third), Some(NamedGroup::X25519));
    }

    /// Test helper: builds a placeholder TLS 1.3 ticket. Its contents are
    /// empty or dummy values; the tests only check whether the cache
    /// keeps it.
    #[cfg(feature = "std")]
    fn tls13_session(now: UnixTime) -> Tls13ClientSessionValue {
        let SupportedCipherSuite::Tls13(tls13_suite) = cipher_suite::TLS13_AES_256_GCM_SHA384
        else {
            unreachable!();
        };
        let server_cert_verifier: Arc<dyn ServerCertVerifier> = Arc::new(DummyServerCertVerifier);
        let resolves_client_cert: Arc<dyn ResolvesClientCert> = Arc::new(DummyResolvesClientCert);
        Tls13ClientSessionValue::new(
            tls13_suite,
            Arc::new(PayloadU16::empty()),
            &[],
            CertificateChain::default(),
            &server_cert_verifier,
            &resolves_client_cert,
            now,
            0,
            0,
            0,
        )
    }

    #[derive(Debug)]
    struct DummyServerCertVerifier;

    impl ServerCertVerifier for DummyServerCertVerifier {
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, Error> {
            unreachable!()
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            unreachable!()
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            unreachable!()
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            unreachable!()
        }
    }

    #[derive(Debug)]
    struct DummyResolvesClientCert;

    impl ResolvesClientCert for DummyResolvesClientCert {
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn resolve(
            &self,
            _root_hint_subjects: &[&[u8]],
            _sigschemes: &[SignatureScheme],
        ) -> Option<Arc<sign::CertifiedKey>> {
            unreachable!()
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn has_certs(&self) -> bool {
            unreachable!()
        }
    }
}
