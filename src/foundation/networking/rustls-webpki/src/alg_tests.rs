// Copyright 2015 Brian Smith.
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHORS DISCLAIM ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

#![allow(clippy::duplicate_mod)]

use alloc::string::String;
use alloc::vec::Vec;

use base64::{Engine as _, engine::general_purpose};
use pki_types::alg_id;

use crate::error::{DerTypeId, Error, UnsupportedSignatureAlgorithmForPublicKeyContext};
use crate::verify_cert::Budget;
use crate::{der, signed_data};

use super::{
    OK_IF_POINT_COMPRESSION_SUPPORTED, SUPPORTED_ALGORITHMS_IN_TESTS, invalid_rsa_signature,
    maybe_rsa, unsupported, unsupported_for_ecdsa, unsupported_for_rsa,
};

macro_rules! test_file_bytes {
    ( $file_name:expr ) => {
        include_bytes!(concat!(
            "../third-party/chromium/data/verify_signed_data/",
            $file_name
        ))
    };
}

// TODO: The expected results need to be modified for SHA-1 deprecation.

fn test_verify_signed_data(file_contents: &[u8]) -> Result<(), Error> {
    let tsd = parse_test_signed_data(file_contents);
    let spki_value = untrusted::Input::from(&tsd.spki);
    let spki_value = spki_value
        .read_all(Error::BadDer, |input| {
            der::expect_tag(input, der::Tag::Sequence)
        })
        .unwrap();

    // we can't use `parse_signed_data` because it requires `data`
    // to be an ASN.1 SEQUENCE, and that isn't the case with
    // Chromium's test data. TODO: The test data set should be
    // expanded with SEQUENCE-wrapped data so that we can actually
    // test `parse_signed_data`.

    let algorithm = untrusted::Input::from(&tsd.algorithm);
    let algorithm = algorithm
        .read_all(
            Error::TrailingData(DerTypeId::SignatureAlgorithm),
            |input| der::expect_tag(input, der::Tag::Sequence),
        )
        .unwrap();

    let signature = untrusted::Input::from(&tsd.signature);
    let signature = signature
        .read_all(Error::TrailingData(DerTypeId::Signature), |input| {
            der::bit_string_with_no_unused_bits(input)
        })
        .unwrap();

    let signed_data = signed_data::SignedData {
        data: untrusted::Input::from(&tsd.data),
        algorithm,
        signature,
    };

    signed_data::verify_signed_data(
        SUPPORTED_ALGORITHMS_IN_TESTS,
        spki_value,
        &signed_data,
        &mut Budget::default(),
    )
}

fn test_verify_signed_data_signature_outer(file_contents: &[u8]) -> Result<(), Error> {
    let tsd = parse_test_signed_data(file_contents);
    let signature = untrusted::Input::from(&tsd.signature);
    signature.read_all(Error::TrailingData(DerTypeId::Signature), |input| {
        der::bit_string_with_no_unused_bits(input)
    })?;

    Ok(())
}

fn test_parse_spki_bad_outer(file_contents: &[u8]) -> Result<(), Error> {
    let tsd = parse_test_signed_data(file_contents);
    let spki = untrusted::Input::from(&tsd.spki);
    spki.read_all(Error::BadDer, |input| {
        der::expect_tag(input, der::Tag::Sequence)
    })?;

    Ok(())
}

// XXX: Some of the BadDer tests should have better error codes, maybe?

// XXX: We should have a variant of this test with a SHA-256 digest that gives
// `Error::UnsupportedSignatureAlgorithmForPublicKey`.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_prime256v1_sha512_spki_params_null_111.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_prime256v1_sha512_unused_bits_signature_124.rs"));

// XXX: We should have a variant of this test with a SHA-256 digest that gives
// `Error::UnsupportedSignatureAlgorithmForPublicKey`.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_prime256v1_sha512_using_ecdh_key_136.rs"));

// XXX: We should have a variant of this test with a SHA-256 digest that gives
// `Error::UnsupportedSignatureAlgorithmForPublicKey`.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_prime256v1_sha512_using_ecmqv_key_151.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_prime256v1_sha512_using_rsa_algorithm_164.rs"));

// XXX: We should have a variant of this test with a SHA-256 digest that gives
// `Error::InvalidSignatureForPublicKey`.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_prime256v1_sha512_wrong_signature_format_179.rs"));

// Differs from Chromium because we don't support P-256 with SHA-512.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_prime256v1_sha512_193.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_secp384r1_sha256_corrupted_data_204.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_secp384r1_sha256_214.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_using_rsa_key_222.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha1_bad_key_der_length_237.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha1_bad_key_der_null_245.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha1_key_params_absent_253.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha1_using_pss_key_no_params_263.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha1_wrong_algorithm_275.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha1_283.rs"));

// XXX: RSA PKCS#1 with SHA-1 is a supported algorithm, but we only accept
// 2048-8192 bit keys, and this test file is using a 1024 bit key. Thus,
// our results differ from Chromium's. TODO: this means we need a 2048+ bit
// version of this test.

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha256_298.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha256_key_encoded_ber_306.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha256_spki_non_null_params_314.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha256_using_ecdsa_algorithm_327.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pkcs1_sha256_using_id_ea_rsa_344.rs"));

// Chromium's PSS test are for parameter combinations we don't support.

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha1_salt20_using_pss_key_no_params_357.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha1_salt20_using_pss_key_with_null_params_369.rs"));
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha1_salt20_380.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha1_wrong_salt_390.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha256_mgf1_sha512_salt33_400.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha256_salt10_using_pss_key_with_params_410.rs"));
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha256_salt10_using_pss_key_with_wrong_params_421.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha256_salt10_433.rs"));

// Our PSS tests that should work.

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha256_salt32_445.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha384_salt48_453.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha512_salt64_461.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha256_salt32_corrupted_data_469.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha384_salt48_corrupted_data_479.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_pss_sha512_salt64_corrupted_data_489.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa_using_ec_key_499.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_rsa2048_pkcs1_sha512_510.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_prime256v1_sha256_518.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_prime256v1_sha256_compressed_526.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/test_ecdsa_prime256v1_sha256_spki_inside_spki_536.rs"));

#[cfg(all(feature = "aws-lc-rs", not(feature = "aws-lc-rs-fips")))]
mod ml_dsa {
    use pki_types::CertificateDer;
    use pki_types::pem::PemObject;
    use untrusted::Input;

    use super::*;
    use crate::cert::Cert;
    use crate::signed_data::verify_signed_data;

    /// From <https://www.ietf.org/archive/id/draft-ietf-lamps-dilithium-certificates-11.html#name-example-certificates>.
    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/self_signed_ml_dsa_44_557.rs"));

    /// From <https://www.ietf.org/archive/id/draft-ietf-lamps-dilithium-certificates-11.html#name-example-certificates>.
    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/self_signed_ml_dsa_65_572.rs"));

    /// From <https://www.ietf.org/archive/id/draft-ietf-lamps-dilithium-certificates-11.html#name-example-certificates>.
    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustls-webpki/functions/src/alg_tests/self_signed_ml_dsa_87_587.rs"));
}

struct TestSignedData {
    spki: Vec<u8>,
    data: Vec<u8>,
    algorithm: Vec<u8>,
    signature: Vec<u8>,
}

fn parse_test_signed_data(file_contents: &[u8]) -> TestSignedData {
    let mut lines = core::str::from_utf8(file_contents).unwrap().lines();
    let spki = read_pem_section(&mut lines, "PUBLIC KEY");
    let algorithm = read_pem_section(&mut lines, "ALGORITHM");
    let data = read_pem_section(&mut lines, "DATA");
    let signature = read_pem_section(&mut lines, "SIGNATURE");

    TestSignedData {
        spki,
        data,
        algorithm,
        signature,
    }
}

use alloc::str::Lines;

fn read_pem_section(lines: &mut Lines<'_>, section_name: &str) -> Vec<u8> {
    // Skip comments and header
    let begin_section = format!("-----BEGIN {section_name}-----");
    loop {
        let line = lines.next().unwrap();
        if line == begin_section {
            break;
        }
    }

    let mut base64 = String::new();

    let end_section = format!("-----END {section_name}-----");
    loop {
        let line = lines.next().unwrap();
        if line == end_section {
            break;
        }
        base64.push_str(line);
    }

    general_purpose::STANDARD.decode(&base64).unwrap()
}
