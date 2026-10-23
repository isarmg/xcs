//! Offline regressions for RUSTSEC-2026-0382 and the matching root-store update.
//!
//! The only private key here is a newly generated, deliberately public localhost
//! test key. Distrusted inputs contain public SPKIs only; see docs/security/TLS_TEST_FIXTURES.md.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rustls_pki_types::{CertificateDer, Der, PrivatePkcs8KeyDer, TrustAnchor, UnixTime};
use webpki::{EndEntityCert, KeyUsage, anchor_from_trusted_cert};

const ROOT: &[u8] = include_bytes!("fixtures/tls/root.der");
const INTERMEDIATE: &[u8] = include_bytes!("fixtures/tls/intermediate.der");
const LEAF: &[u8] = include_bytes!("fixtures/tls/localhost.der");
const LEAF_TEST_KEY: &[u8] = include_bytes!("fixtures/tls/localhost-test-key.pkcs8.der");
const REMOVED_ROOT: &[u8] = include_bytes!("fixtures/tls/emsign-ecc-root-ca---g3.der");

macro_rules! distrusted_spki {
    ($name:literal) => {
        (
            $name,
            include_bytes!(concat!(
                "../../../src/foundation/networking/rustls-webpki/src/data/distrusted/",
                $name,
                ".spki.der"
            )) as &[u8],
        )
    };
}

// Keep all eight advisory keys explicit so missing denylist entries cannot hide
// behind an iteration over whichever entries the implementation happens to use.
const DISTRUSTED_SPKIS: &[(&str, &[u8])] = &[
    distrusted_spki!("EMDVTLSCAG3B"),
    distrusted_spki!("EMDVTLSCAG3C"),
    distrusted_spki!("EMEVTLSCAG3B"),
    distrusted_spki!("EMEVTLSCAG3C"),
    distrusted_spki!("EMOVCSCAG3B"),
    distrusted_spki!("EMOVCSCAG3C"),
    distrusted_spki!("EMOVTLSCAG3B"),
    distrusted_spki!("EMOVTLSCAG3C"),
];

fn root_anchor() -> TrustAnchor<'static> {
    anchor_from_trusted_cert(&CertificateDer::from(ROOT))
        .unwrap()
        .to_owned()
}

fn verify_chain(
    leaf: &[u8],
    intermediate: &[u8],
    anchor: TrustAnchor<'_>,
) -> Result<(), webpki::Error> {
    let leaf = CertificateDer::from(leaf);
    EndEntityCert::try_from(&leaf)?.verify_for_usage(
        webpki::ALL_VERIFICATION_ALGS,
        &[anchor],
        &[CertificateDer::from(intermediate)],
        UnixTime::since_unix_epoch(Duration::from_secs(1_792_454_400)),
        KeyUsage::server_auth(),
        None,
        None,
    )?;
    Ok(())
}

#[test]
fn ordinary_chain_still_verifies_and_invalid_signatures_still_fail() {
    verify_chain(LEAF, INTERMEDIATE, root_anchor()).unwrap();
    let mut invalid_signature = LEAF.to_vec();
    *invalid_signature.last_mut().unwrap() ^= 1;
    assert_eq!(
        verify_chain(&invalid_signature, INTERMEDIATE, root_anchor()),
        Err(webpki::Error::InvalidSignatureForPublicKey)
    );
}

fn assert_distrusted(result: Result<(), webpki::Error>, key: &str, position: &str) {
    // Compare the error name so this test also compiles against 0.103.15 when
    // demonstrating the behavioral regression, rather than failing to compile
    // only because the old release has no CertDistrusted enum variant.
    assert_eq!(
        format!(
            "{:?}",
            result.expect_err("distrusted public key was accepted")
        ),
        "CertDistrusted",
        "{key} at {position} must fail specifically at the distrust check"
    );
}

#[test]
fn every_advisory_spki_is_distrusted_in_leaf_certificates() {
    for &(name, spki) in DISTRUSTED_SPKIS {
        assert_distrusted(
            verify_chain(
                &certificate_with_spki(LEAF, spki),
                INTERMEDIATE,
                root_anchor(),
            ),
            name,
            "leaf",
        );
    }
}

#[test]
fn every_advisory_spki_is_distrusted_in_intermediate_certificates() {
    for &(name, spki) in DISTRUSTED_SPKIS {
        assert_distrusted(
            verify_chain(
                LEAF,
                &certificate_with_spki(INTERMEDIATE, spki),
                root_anchor(),
            ),
            name,
            "intermediate",
        );
    }
}

#[test]
fn every_advisory_spki_is_distrusted_even_as_an_explicit_trust_anchor() {
    for &(name, spki) in DISTRUSTED_SPKIS {
        let mut anchor = root_anchor();
        anchor.subject_public_key_info = Der::from_slice(spki);
        assert_distrusted(
            verify_chain(LEAF, INTERMEDIATE, anchor),
            name,
            "trust anchor",
        );
    }
}

#[test]
fn compromised_emsign_g3_root_is_not_in_the_default_root_store() {
    let certificate = CertificateDer::from(REMOVED_ROOT);
    let removed = anchor_from_trusted_cert(&certificate).unwrap();
    assert!(!webpki_roots::TLS_SERVER_ROOTS.is_empty());
    assert!(
        webpki_roots::TLS_SERVER_ROOTS
            .iter()
            .all(|root| { root.subject_public_key_info != removed.subject_public_key_info }),
        "emSign ECC Root CA - G3 must not be trusted, even under another subject"
    );
}

#[tokio::test]
async fn reqwest_tls_accepts_a_valid_localhost_chain_with_an_explicit_test_root() {
    let (address, server) = tls_server(INTERMEDIATE.to_vec());
    let response = trusted_test_client()
        .get(format!("https://{address}/"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "ok");
    assert!(
        server
            .join()
            .unwrap()
            .unwrap()
            .starts_with("GET / HTTP/1.1\r\n")
    );
}

#[tokio::test]
async fn production_storage_client_rejects_an_untrusted_localhost_chain() {
    let (address, server) = tls_server(INTERMEDIATE.to_vec());
    let client = storage_client::Client::connect(&format!("https://{address}")).unwrap();
    let error = tokio::time::timeout(
        Duration::from_secs(10),
        client.request("GET", "/api/tls-regression", None),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(error_chain(&error).contains("UnknownIssuer"), "{error:?}");
    assert!(
        server.join().unwrap().is_err(),
        "HTTP request must not be sent"
    );
}

#[tokio::test]
async fn reqwest_tls_rejects_every_distrusted_intermediate_before_sending_http() {
    for &(name, spki) in DISTRUSTED_SPKIS {
        let (address, server) = tls_server(certificate_with_spki(INTERMEDIATE, spki));
        let error = trusted_test_client()
            .get(format!("https://{address}/"))
            .send()
            .await
            .unwrap_err();
        assert!(
            error_chain(&error).contains("CertDistrusted"),
            "{name} must reach the distrust check through reqwest/rustls: {error:?}"
        );
        assert!(
            server.join().unwrap().is_err(),
            "HTTP request must not be sent"
        );
    }
}

fn trusted_test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .tls_built_in_root_certs(false)
        .add_root_certificate(reqwest::Certificate::from_der(ROOT).unwrap())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
}

fn error_chain(error: &dyn std::error::Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(error) = source {
        message.push_str(&format!(": {error}"));
        source = error.source();
    }
    message
}

fn tls_server(intermediate: Vec<u8>) -> (SocketAddr, thread::JoinHandle<std::io::Result<String>>) {
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![
            CertificateDer::from(LEAF),
            CertificateDer::from(intermediate),
        ],
        PrivatePkcs8KeyDer::from(LEAF_TEST_KEY).into(),
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let socket = loop {
            match listener.accept() {
                Ok((socket, _)) => break socket,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(std::io::ErrorKind::TimedOut.into());
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error),
            }
        };
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;
        socket.set_write_timeout(Some(Duration::from_secs(5)))?;
        let connection = rustls::ServerConnection::new(Arc::new(config)).unwrap();
        let mut stream = rustls::StreamOwned::new(connection, socket);
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte)?;
            request.push(byte[0]);
            assert!(request.len() < 8192, "unbounded HTTP request");
        }
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")?;
        stream.flush()?;
        Ok(String::from_utf8(request).unwrap())
    });
    (address, server)
}

// Tiny DER surgery over our own fixed fixtures, not a general-purpose parser.
// Changing a public key invalidates the original certificate signature. That is
// intentional: the advisory must reject the key before issuer/signature checks.
// The positive and tampered-signature controls above verify real signatures.
fn certificate_with_spki(certificate: &[u8], spki_value: &[u8]) -> Vec<u8> {
    let outer = sequence_parts(certificate);
    let mut tbs = sequence_parts(outer[0]);
    let spki = sequence(spki_value);
    tbs[6] = &spki;
    sequence(
        &[
            sequence(&tbs.concat()),
            outer[1].to_vec(),
            outer[2].to_vec(),
        ]
        .concat(),
    )
}

fn sequence_parts(encoded: &[u8]) -> Vec<&[u8]> {
    assert_eq!(encoded[0], 0x30);
    let (_, mut remaining, trailing) = der_tlv(encoded);
    assert!(trailing.is_empty());
    let mut parts = Vec::new();
    while !remaining.is_empty() {
        let (full, _, rest) = der_tlv(remaining);
        parts.push(full);
        remaining = rest;
    }
    parts
}

fn der_tlv(encoded: &[u8]) -> (&[u8], &[u8], &[u8]) {
    let (header, length) = if encoded[1] < 0x80 {
        (2, usize::from(encoded[1]))
    } else {
        let count = usize::from(encoded[1] & 0x7f);
        assert!((1..=2).contains(&count));
        let length = encoded[2..2 + count]
            .iter()
            .fold(0, |length, byte| (length << 8) | usize::from(*byte));
        (2 + count, length)
    };
    let end = header + length;
    (&encoded[..end], &encoded[header..end], &encoded[end..])
}

fn sequence(value: &[u8]) -> Vec<u8> {
    let length = value.len();
    assert!(length <= usize::from(u16::MAX));
    let mut encoded = vec![0x30];
    if length < 128 {
        encoded.push(length as u8);
    } else if length <= usize::from(u8::MAX) {
        encoded.extend_from_slice(&[0x81, length as u8]);
    } else {
        encoded.push(0x82);
        encoded.extend_from_slice(&(length as u16).to_be_bytes());
    }
    encoded.extend_from_slice(value);
    encoded
}
