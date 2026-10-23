use alloc::boxed::Box;
use alloc::vec;

use super::ServerConnectionData;
use crate::common_state::Context;
use crate::enums::{CipherSuite, SignatureScheme};
use crate::msgs::base::PayloadU16;
use crate::msgs::enums::{Compression, NamedGroup};
use crate::msgs::handshake::{
    ClientExtensions, ClientHelloPayload, HandshakeMessagePayload, HandshakePayload, KeyShareEntry,
    Random, ServerNamePayload, SessionId, SupportedProtocolVersions,
};
use crate::msgs::message::{Message, MessagePayload};
use crate::{CommonState, Error, PeerIncompatible, PeerMisbehaved, ProtocolVersion, Side};

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/null_compression_required_16.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/server_ignores_sni_with_ip_address_27.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/server_rejects_sni_with_illegal_dns_name_35.rs"));

fn test_process_client_hello(hello: ClientHelloPayload) -> Result<(), Error> {
    let m = Message {
        version: ProtocolVersion::TLSv1_2,
        payload: MessagePayload::handshake(HandshakeMessagePayload(HandshakePayload::ClientHello(
            hello,
        ))),
    };
    super::hs::process_client_hello(
        &m,
        false,
        &mut Context {
            common: &mut CommonState::new(Side::Server),
            data: &mut ServerConnectionData::default(),
            sendable_plaintext: None,
        },
    )
    .map(|_| ())
}

#[macro_rules_attribute::apply(test_for_each_provider)]
mod tests {
    use alloc::vec::Vec;

    use super::super::*;
    use crate::common_state::KxState;
    use crate::crypto::{
        ActiveKeyExchange, CryptoProvider, KeyExchangeAlgorithm, SupportedKxGroup,
    };
    use crate::enums::CertificateType;
    use crate::msgs::codec::{Codec, Reader};
    use crate::msgs::handshake::{
        PresharedKeyBinder, PresharedKeyIdentity, PresharedKeyOffer, PskKeyExchangeModes,
    };
    use crate::msgs::message::HEADER_SIZE;
    use crate::msgs::persist;
    use crate::pki_types::pem::PemObject;
    use crate::pki_types::{CertificateDer, PrivateKeyDer};
    use crate::server::{AlwaysResolvesServerRawPublicKeys, ServerConfig, ServerConnection};
    use crate::sign::CertifiedKey;
    use crate::sync::Arc;
    use crate::tls13::key_schedule::KeyScheduleEarly;
    use crate::{
        CipherSuiteCommon, SupportedCipherSuite, Tls12CipherSuite, Tls13CipherSuite, version,
    };

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/test_server_rejects_no_extended_master_secret_extension_when_require_ems_or_fips_91.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/server_picks_ffdhe_group_when_clienthello_has_no_ffdhe_group_in_groups_ext_130.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/server_picks_ffdhe_group_when_clienthello_has_no_groups_ext_150.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/server_accepts_client_with_no_ecpoints_extension_and_only_ffdhe_cipher_suites_171.rs"));

    fn server_chooses_ffdhe_group_for_client_hello(
        mut conn: ServerConnection,
        client_hello: ClientHelloPayload,
    ) {
        let ch = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::handshake(HandshakeMessagePayload(
                HandshakePayload::ClientHello(client_hello),
            )),
        };
        conn.read_tls(&mut ch.into_wire_bytes().as_slice())
            .unwrap();
        conn.process_new_packets().unwrap();

        let KxState::Start(skxg) = &conn.kx_state else {
            panic!("unexpected kx_state");
        };
        assert_eq!(skxg.name(), FAKE_FFDHE_GROUP.name());
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/test_server_requiring_rpk_client_rejects_x509_client_212.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/test_rpk_only_server_rejects_x509_only_client_232.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/second_client_hello_cannot_withdraw_psk_offer_252.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/second_client_hello_cannot_change_cipher_suite_306.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/early_data_accepted_for_first_psk_identity_358.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls/functions/src/server/test/early_data_rejected_for_later_psk_identity_365.rs"));

    /// Offer a resumable PSK at `index` (preceded by unknown identities) along with an
    /// early data request, returning the selected PSK index and whether early data was accepted.
    fn resume_with_early_data(index: usize) -> (Option<u16>, bool) {
        // ensures X25519 is offered irrespective of cfg(feature = "fips"), to match
        // `minimal_client_hello()`
        let provider = CryptoProvider {
            kx_groups: vec![super::provider::kx_group::X25519],
            ..super::provider::default_provider()
        };
        let mut config = ServerConfig::builder_with_provider(provider.into())
            .with_protocol_versions(&[&version::TLS13])
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(server_cert(), server_key())
            .unwrap();
        config.max_early_data_size = 1024;

        let SupportedCipherSuite::Tls13(suite) =
            super::provider::cipher_suite::TLS13_AES_128_GCM_SHA256
        else {
            unreachable!();
        };

        let secret = [0x42u8; 32];
        let ticket = b"resumable ticket";
        let session = persist::ServerSessionValue::new(
            None,
            ProtocolVersion::TLSv1_3,
            suite.common.suite,
            &secret,
            None,
            None,
            vec![],
            config.current_time().unwrap(),
            0,
        );
        assert!(
            config
                .session_storage
                .put(ticket.to_vec(), session.get_encoding())
        );

        let binder_len = suite.common.hash_provider.output_len();
        let mut identities = vec![PresharedKeyIdentity::new(b"unknown ticket".to_vec(), 0); index];
        identities.push(PresharedKeyIdentity::new(ticket.to_vec(), 0));
        let binders = vec![PresharedKeyBinder::from(vec![0u8; binder_len]); identities.len()];

        let mut ch = minimal_client_hello();
        ch.cipher_suites = vec![suite.common.suite];
        ch.extensions.preshared_key_modes = Some(PskKeyExchangeModes {
            psk_dhe: true,
            psk: false,
        });
        ch.extensions.early_data_request = Some(());
        ch.extensions.preshared_key_offer = Some(PresharedKeyOffer {
            identities,
            binders,
        });
        let mut hmp = HandshakeMessagePayload(HandshakePayload::ClientHello(ch));

        // compute a genuine binder for the resumable PSK over the final ClientHello
        let key_schedule = KeyScheduleEarly::new(suite, &secret);
        let handshake_hash = suite
            .common
            .hash_provider
            .hash(&hmp.encoding_for_binder_signing());
        let binder = key_schedule.resumption_psk_binder_key_and_sign_verify_data(&handshake_hash);
        let HandshakePayload::ClientHello(ch) = &mut hmp.0 else {
            unreachable!();
        };
        ch.preshared_key_offer
            .as_mut()
            .unwrap()
            .binders[index] = PresharedKeyBinder::from(binder.as_ref().to_vec());

        let ch = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::handshake(hmp),
        };
        let mut conn = ServerConnection::new(config.into()).unwrap();
        conn.read_tls(&mut ch.into_wire_bytes().as_slice())
            .unwrap();
        conn.process_new_packets().unwrap();

        let mut flight = vec![];
        conn.write_tls(&mut flight).unwrap();
        let mut r = Reader::init(&flight[HEADER_SIZE..]);
        let HandshakeMessagePayload(HandshakePayload::ServerHello(server_hello)) =
            HandshakeMessagePayload::read(&mut r).unwrap()
        else {
            panic!("expected ServerHello");
        };

        (
            server_hello.extensions.preshared_key,
            conn.early_data().is_some(),
        )
    }

    static TLS13_AES_128_GCM_SHA256_ALT: SupportedCipherSuite =
        SupportedCipherSuite::Tls13(&TLS13_AES_128_GCM_SHA256_ALT_INNER);

    /// Differs from `TLS13_AES_128_GCM_SHA256` only in its code point: same hash, same everything else.
    static TLS13_AES_128_GCM_SHA256_ALT_INNER: Tls13CipherSuite =
        match &super::provider::cipher_suite::TLS13_AES_128_GCM_SHA256 {
            SupportedCipherSuite::Tls13(provider) => Tls13CipherSuite {
                common: CipherSuiteCommon {
                    suite: CipherSuite::Unknown(0xff14),
                    ..provider.common
                },
                ..**provider
            },
            _ => unreachable!(),
        };

    fn server_config_for_rpk() -> ServerConfig {
        let x25519_provider = CryptoProvider {
            kx_groups: vec![super::provider::kx_group::X25519],
            ..super::provider::default_provider()
        };
        ServerConfig::builder_with_provider(x25519_provider.into())
            .with_protocol_versions(&[&version::TLS13])
            .unwrap()
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(AlwaysResolvesServerRawPublicKeys::new(Arc::new(
                server_certified_key(),
            ))))
    }

    fn server_certified_key() -> CertifiedKey {
        let key = super::provider::default_provider()
            .key_provider
            .load_private_key(server_key())
            .unwrap();
        let public_key_as_cert = vec![CertificateDer::from(
            key.public_key()
                .unwrap()
                .as_ref()
                .to_vec(),
        )];
        CertifiedKey::new(public_key_as_cert, key)
    }

    fn server_key() -> PrivateKeyDer<'static> {
        PrivateKeyDer::from_pem_reader(
            &mut include_bytes!("../../../../../../src/foundation/test-ca/rsa-2048/end.key").as_slice(),
        )
        .unwrap()
    }

    fn server_cert() -> Vec<CertificateDer<'static>> {
        vec![
            CertificateDer::from(&include_bytes!("../../../../../../src/foundation/test-ca/rsa-2048/end.der")[..]),
            CertificateDer::from(&include_bytes!("../../../../../../src/foundation/test-ca/rsa-2048/inter.der")[..]),
        ]
    }

    fn ffdhe_provider() -> CryptoProvider {
        CryptoProvider {
            kx_groups: vec![FAKE_FFDHE_GROUP],
            cipher_suites: vec![TLS_DHE_RSA_WITH_AES_128_GCM_SHA256],
            ..super::provider::default_provider()
        }
    }

    static FAKE_FFDHE_GROUP: &'static dyn SupportedKxGroup = &FakeFfdheGroup;

    #[derive(Debug)]
    struct FakeFfdheGroup;

    impl SupportedKxGroup for FakeFfdheGroup {
        fn name(&self) -> NamedGroup {
            NamedGroup::FFDHE2048
        }

        fn start(&self) -> Result<Box<dyn ActiveKeyExchange>, Error> {
            Ok(Box::new(ActiveFakeFfdhe))
        }
    }

    #[derive(Debug)]
    struct ActiveFakeFfdhe;

    impl ActiveKeyExchange for ActiveFakeFfdhe {
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn complete(
            self: Box<Self>,
            _peer_pub_key: &[u8],
        ) -> Result<crate::crypto::SharedSecret, Error> {
            todo!()
        }

        fn pub_key(&self) -> &[u8] {
            b"ActiveFakeFfdhe pub key"
        }

        fn group(&self) -> NamedGroup {
            NamedGroup::FFDHE2048
        }
    }

    static TLS_DHE_RSA_WITH_AES_128_GCM_SHA256: SupportedCipherSuite =
        SupportedCipherSuite::Tls12(&TLS12_DHE_RSA_WITH_AES_128_GCM_SHA256);

    static TLS12_DHE_RSA_WITH_AES_128_GCM_SHA256: Tls12CipherSuite =
        match &super::provider::cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256 {
            SupportedCipherSuite::Tls12(provider) => Tls12CipherSuite {
                common: CipherSuiteCommon {
                    suite: CipherSuite::TLS_DHE_RSA_WITH_AES_128_GCM_SHA256,
                    ..provider.common
                },
                kx: KeyExchangeAlgorithm::DHE,
                ..**provider
            },
            _ => unreachable!(),
        };
}

fn minimal_client_hello() -> ClientHelloPayload {
    ClientHelloPayload {
        client_version: ProtocolVersion::TLSv1_3,
        random: Random::from([0u8; 32]),
        session_id: SessionId::empty(),
        cipher_suites: vec![
            CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
            CipherSuite::TLS13_AES_128_GCM_SHA256,
        ],
        compression_methods: vec![Compression::Null],
        extensions: Box::new(ClientExtensions {
            signature_schemes: Some(vec![SignatureScheme::RSA_PSS_SHA256]),
            named_groups: Some(vec![NamedGroup::X25519, NamedGroup::secp256r1]),
            supported_versions: Some(SupportedProtocolVersions {
                tls12: true,
                tls13: true,
            }),
            key_shares: Some(vec![KeyShareEntry {
                group: NamedGroup::X25519,
                payload: PayloadU16::new(vec![0xab; 32]),
            }]),
            extended_master_secret_request: Some(()),
            ..ClientExtensions::default()
        }),
    }
}
