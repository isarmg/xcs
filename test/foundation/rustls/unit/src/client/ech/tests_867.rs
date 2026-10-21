#[cfg(test)]
mod tests {
    use std::string::String;

    use super::*;
    use crate::enums::CipherSuite;
    use crate::msgs::enums::{Compression, HpkeAead, HpkeKdf};
    use crate::msgs::handshake::{Random, ServerExtensions, SessionId};

    #[test]
    fn server_hello_conf_alters_server_hello_random() {
        let server_hello = ServerHelloPayload {
            legacy_version: ProtocolVersion::TLSv1_2,
            random: Random([0xffu8; 32]),
            session_id: SessionId::empty(),
            cipher_suite: CipherSuite::TLS13_AES_256_GCM_SHA384,
            compression_method: Compression::Null,
            extensions: Box::new(ServerExtensions::default()),
        };
        let message = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::handshake(HandshakeMessagePayload(
                HandshakePayload::ServerHello(server_hello.clone()),
            )),
        };
        let Message {
            payload:
                MessagePayload::Handshake {
                    encoded: server_hello_encoded_before,
                    ..
                },
            ..
        } = &message
        else {
            unreachable!("ServerHello is a handshake message");
        };

        let message = EchState::server_hello_conf(&server_hello, server_hello_encoded_before);

        let Message {
            payload:
                MessagePayload::Handshake {
                    encoded: server_hello_encoded_after,
                    ..
                },
            ..
        } = &message
        else {
            unreachable!("ServerHello is a handshake message");
        };

        assert_eq!(
            std::format!("{server_hello_encoded_before:x?}"),
            "020000280303ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff001302000000",
            "beforehand eight bytes at end of Random should be 0xff here ^^^^^^^^^^^^^^^^            "
        );
        assert_eq!(
            std::format!("{server_hello_encoded_after:x?}"),
            "020000280303ffffffffffffffffffffffffffffffffffffffffffffffff0000000000000000001302000000",
            "                          afterwards those bytes are zeroed ^^^^^^^^^^^^^^^^            "
        );
    }

    #[test]
    fn inner_client_hello_length_conceals_inner_name_length() {
        let base_inner_len = inner_hello_encoding_for_name(dns_name_of_len(1), true).len();
        assert!(
            base_inner_len % 32 == 0,
            "inner hello length must be 32-byte padded"
        );
        assert!(
            base_inner_len >= 256,
            "inner hello must include inner name and its padding"
        );

        for inner_name_len in 1..251 {
            assert_eq!(
                inner_hello_encoding_for_name(dns_name_of_len(inner_name_len), true).len(),
                base_inner_len,
                "all inner hello lengths must be invariant wrt inner name length"
            );
        }
    }

    #[test]
    fn inner_client_hello_length_does_not_leak_length_of_omitted_inner_name() {
        let base_inner_len = inner_hello_encoding_for_name(dns_name_of_len(1), false).len();
        assert!(
            base_inner_len % 32 == 0,
            "inner hello length must be 32-byte padded"
        );
        assert!(
            base_inner_len >= 256,
            "inner hello must include maximum_name_length bytes of padding"
        );

        for inner_name_len in 1..251 {
            assert_eq!(
                inner_hello_encoding_for_name(dns_name_of_len(inner_name_len), false).len(),
                base_inner_len,
                "all inner hello lengths must be invariant wrt inner name length"
            );
        }
    }

    #[macro_rules_attribute::apply(test_for_each_provider)]
    mod hello_retry_request {
        use super::*;
        use crate::common_state::Side;
        use crate::crypto::SharedSecret;
        use crate::msgs::base::PayloadU16;
        use crate::msgs::handshake::{HelloRetryRequestExtensions, SupportedProtocolVersions};
        use crate::tls13::key_schedule::KeySchedulePreHandshake;

        #[test]
        fn ech_rejection_in_server_hello_after_acceptance_in_hello_retry_request() {
            let suite = tls13_suite();
            let hash = suite.common.hash_provider;
            let mut state = ech_state();

            // the server accepts our offer in its hello retry request...
            state
                .ech_hello(outer_hello(), None, &None)
                .unwrap();
            let hrr = accepting_hello_retry_request(&state, suite);
            assert!(
                state
                    .confirm_hrr_acceptance(&hrr, suite, &mut CommonState::new(Side::Client))
                    .unwrap()
            );
            state.transcript_hrr_update(hash, &hrr_message(&hrr));
            state
                .ech_hello(outer_hello(), Some(&hrr), &None)
                .unwrap();

            // ...but its server hello does not confirm that acceptance
            let server_hello = server_hello(suite);
            let mut server_name = inner_name();
            assert_eq!(
                state
                    .confirm_acceptance(
                        &mut handshake_key_schedule(suite),
                        &server_hello,
                        &encode(&server_hello),
                        hash,
                        &mut server_name,
                        &mut CommonState::new(Side::Client),
                    )
                    .err(),
                Some(PeerMisbehaved::EchAcceptanceVariedAfterRetry.into())
            );
        }

        #[test]
        fn ech_acceptance_in_server_hello_after_rejection_in_hello_retry_request() {
            let suite = tls13_suite();
            let hash = suite.common.hash_provider;
            let mut key_schedule = handshake_key_schedule(suite);

            // without a hello retry request, a confirming server hello accepts ECH
            let mut state = ech_state();
            state
                .ech_hello(outer_hello(), None, &None)
                .unwrap();
            let server_hello = confirming_server_hello(&state, &mut key_schedule, suite);
            let mut server_name = inner_name();
            assert!(
                state
                    .confirm_acceptance(
                        &mut key_schedule,
                        &server_hello,
                        &encode(&server_hello),
                        hash,
                        &mut server_name,
                        &mut CommonState::new(Side::Client),
                    )
                    .unwrap()
                    .is_some()
            );
            assert_eq!(server_name, inner_name());

            // but after a rejecting hello retry request, a confirming server hello cannot revive it
            let mut state = ech_state();
            state
                .ech_hello(outer_hello(), None, &None)
                .unwrap();
            let hrr = hello_retry_request(suite, None);
            assert!(
                !state
                    .confirm_hrr_acceptance(&hrr, suite, &mut CommonState::new(Side::Client))
                    .unwrap()
            );
            state.transcript_hrr_update(hash, &hrr_message(&hrr));
            let server_hello = confirming_server_hello(&state, &mut key_schedule, suite);
            let mut server_name = inner_name();
            assert!(
                state
                    .confirm_acceptance(
                        &mut key_schedule,
                        &server_hello,
                        &encode(&server_hello),
                        hash,
                        &mut server_name,
                        &mut CommonState::new(Side::Client),
                    )
                    .unwrap()
                    .is_none()
            );
            assert_eq!(server_name, ServerName::from(public_name()));
        }

        fn tls13_suite() -> &'static Tls13CipherSuite {
            provider::cipher_suite::TLS13_AES_128_GCM_SHA256
                .tls13()
                .unwrap()
        }

        fn ech_state() -> EchState {
            let config = EchConfig {
                config: EchConfigPayload::V18(EchConfigContents {
                    key_config: HpkeKeyConfig {
                        config_id: 0,
                        kem_id: MockHpke::SUITE.kem,
                        public_key: PayloadU16::new(vec![0; 32]),
                        symmetric_cipher_suites: vec![MockHpke::SUITE.sym],
                    },
                    maximum_name_length: 255,
                    public_name: public_name(),
                    extensions: vec![],
                }),
                suite: &MockHpke,
            };

            EchState::new(&config, inner_name(), false, &FixedRandom, true).unwrap()
        }

        fn inner_name() -> ServerName<'static> {
            ServerName::try_from("inner.example.com").unwrap()
        }

        fn public_name() -> DnsName<'static> {
            DnsName::try_from("public.example.com").unwrap()
        }

        fn outer_hello() -> ClientHelloPayload {
            ClientHelloPayload {
                client_version: ProtocolVersion::TLSv1_2,
                random: Random([0u8; 32]),
                session_id: SessionId::empty(),
                cipher_suites: vec![tls13_suite().common.suite],
                compression_methods: vec![Compression::Null],
                extensions: Box::new(ClientExtensions {
                    server_name: Some(ServerNamePayload::from(&public_name())),
                    supported_versions: Some(SupportedProtocolVersions {
                        tls13: true,
                        tls12: false,
                    }),
                    ..Default::default()
                }),
            }
        }

        fn hello_retry_request(
            suite: &'static Tls13CipherSuite,
            encrypted_client_hello: Option<Payload<'static>>,
        ) -> HelloRetryRequest {
            HelloRetryRequest {
                legacy_version: ProtocolVersion::TLSv1_2,
                session_id: SessionId::empty(),
                cipher_suite: suite.common.suite,
                extensions: HelloRetryRequestExtensions {
                    cookie: Some(PayloadU16::new(vec![1, 2, 3, 4])),
                    supported_versions: Some(ProtocolVersion::TLSv1_3),
                    encrypted_client_hello,
                    ..HelloRetryRequestExtensions::default()
                },
            }
        }

        /// A hello retry request accepting the offer in `state`, see RFC 9849 section 7.2.1.
        fn accepting_hello_retry_request(
            state: &EchState,
            suite: &'static Tls13CipherSuite,
        ) -> HelloRetryRequest {
            let mut hrr = hello_retry_request(suite, Some(Payload::new(vec![0u8; 8])));
            let mut transcript = state
                .inner_hello_transcript
                .clone()
                .start_hash(suite.common.hash_provider);
            transcript.rollup_for_hrr();
            transcript.add_message(&EchState::hello_retry_request_conf(&hrr));
            let confirmation = server_ech_hrr_confirmation_secret(
                suite.hkdf_provider,
                &state.inner_hello_random.0,
                transcript.current_hash(),
            );
            hrr.encrypted_client_hello = Some(Payload::new(confirmation.to_vec()));
            hrr
        }

        fn hrr_message(hrr: &HelloRetryRequest) -> Message<'static> {
            Message {
                version: ProtocolVersion::TLSv1_2,
                payload: MessagePayload::handshake(HandshakeMessagePayload(
                    HandshakePayload::HelloRetryRequest(hrr.clone()),
                )),
            }
        }

        fn server_hello(suite: &'static Tls13CipherSuite) -> ServerHelloPayload {
            ServerHelloPayload {
                legacy_version: ProtocolVersion::TLSv1_2,
                random: Random([0x55u8; 32]),
                session_id: SessionId::empty(),
                cipher_suite: suite.common.suite,
                compression_method: Compression::Null,
                extensions: Box::new(ServerExtensions::default()),
            }
        }

        /// A server hello accepting the offer in `state`, see RFC 9849 section 7.2.
        fn confirming_server_hello(
            state: &EchState,
            key_schedule: &mut KeyScheduleHandshakeStart,
            suite: &'static Tls13CipherSuite,
        ) -> ServerHelloPayload {
            let mut server_hello = server_hello(suite);
            let mut transcript = state
                .inner_hello_transcript
                .clone()
                .start_hash(suite.common.hash_provider);
            transcript.add_message(&EchState::server_hello_conf(
                &server_hello,
                &encode(&server_hello),
            ));
            let confirmation = key_schedule.server_ech_confirmation_secret(
                &state.inner_hello_random.0,
                transcript.current_hash(),
            );
            server_hello.random.0[24..].copy_from_slice(&confirmation);
            server_hello
        }

        fn encode(server_hello: &ServerHelloPayload) -> Payload<'static> {
            Payload::new(
                HandshakeMessagePayload(HandshakePayload::ServerHello(server_hello.clone()))
                    .get_encoding(),
            )
        }

        fn handshake_key_schedule(suite: &'static Tls13CipherSuite) -> KeyScheduleHandshakeStart {
            KeySchedulePreHandshake::new(suite).into_handshake(SharedSecret::from(&[0u8; 32][..]))
        }
    }

    fn inner_hello_encoding_for_name(name: DnsName<'static>, enable_sni: bool) -> Vec<u8> {
        let config = EchConfig {
            config: EchConfigPayload::V18(EchConfigContents {
                key_config: HpkeKeyConfig {
                    config_id: 0,
                    kem_id: MockHpke::SUITE.kem,
                    public_key: PayloadU16::new(vec![0; 32]),
                    symmetric_cipher_suites: vec![],
                },
                maximum_name_length: 255,
                public_name: DnsName::try_from("public").unwrap(),
                extensions: vec![],
            }),
            suite: &MockHpke,
        };

        EchState::new(
            &config,
            ServerName::from(name.clone()),
            false,
            &FixedRandom,
            enable_sni,
        )
        .unwrap()
        .encode_inner_hello(
            &ClientHelloPayload {
                client_version: ProtocolVersion::TLSv1_3,
                random: Random([0u8; 32]),
                session_id: SessionId::empty(),
                cipher_suites: vec![],
                compression_methods: vec![Compression::Null],
                extensions: Box::new(ClientExtensions {
                    server_name: Some(ServerNamePayload::from(&name)),
                    ..Default::default()
                }),
            },
            None,
            &None,
        )
    }

    fn dns_name_of_len(mut len: usize) -> DnsName<'static> {
        let mut s = String::new();
        let labels = len.div_ceil(63);
        for _ in 0..labels {
            let chars = Ord::min(len, 63);
            len -= chars;
            for _ in 0..chars {
                s.push('a');
            }
            if len != 0 {
                s.push('.');
            }
        }
        DnsName::try_from(s).unwrap()
    }

    #[derive(Debug)]
    struct MockHpke;

    impl MockHpke {
        const SUITE: HpkeSuite = HpkeSuite {
            kem: HpkeKem::DHKEM_P256_HKDF_SHA256,
            sym: HpkeSymmetricCipherSuite {
                kdf_id: HpkeKdf::HKDF_SHA256,
                aead_id: HpkeAead::AES_128_GCM,
            },
        };
    }

    impl Hpke for MockHpke {
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn seal(
            &self,
            _info: &[u8],
            _aad: &[u8],
            _plaintext: &[u8],
            _pub_key: &HpkePublicKey,
        ) -> Result<(EncapsulatedSecret, Vec<u8>), Error> {
            todo!()
        }

        fn setup_sealer(
            &self,
            _info: &[u8],
            _pub_key: &HpkePublicKey,
        ) -> Result<(EncapsulatedSecret, Box<dyn HpkeSealer + 'static>), Error> {
            Ok((EncapsulatedSecret(vec![]), Box::new(MockHpkeSealer)))
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn open(
            &self,
            _enc: &EncapsulatedSecret,
            _info: &[u8],
            _aad: &[u8],
            _ciphertext: &[u8],
            _secret_key: &crate::crypto::hpke::HpkePrivateKey,
        ) -> Result<Vec<u8>, Error> {
            todo!()
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn setup_opener(
            &self,
            _enc: &EncapsulatedSecret,
            _info: &[u8],
            _secret_key: &crate::crypto::hpke::HpkePrivateKey,
        ) -> Result<Box<dyn crate::crypto::hpke::HpkeOpener + 'static>, Error> {
            todo!()
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn generate_key_pair(
            &self,
        ) -> Result<(HpkePublicKey, crate::crypto::hpke::HpkePrivateKey), Error> {
            todo!()
        }

        fn suite(&self) -> HpkeSuite {
            Self::SUITE
        }
    }

    #[derive(Debug)]
    struct MockHpkeSealer;

    impl HpkeSealer for MockHpkeSealer {
        fn seal(&mut self, _aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, Error> {
            // ciphertext length as if sealed with the AEAD named in `MockHpke::SUITE`
            Ok(vec![0xff; plaintext.len() + 16])
        }
    }

    #[derive(Debug)]
    struct FixedRandom;

    impl SecureRandom for FixedRandom {
        fn fill(&self, buf: &mut [u8]) -> Result<(), crate::rand::GetRandomFailed> {
            buf.fill(0x55);
            Ok(())
        }
    }
}
