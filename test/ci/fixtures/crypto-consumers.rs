//! Independent SHA-2/HMAC vectors and optional BLAKE3 trait integration.
use sha2::{cryptographic_core::Digest, Sha256, Sha512};

fn decode(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn sha2_known_vectors() {
    assert_eq!(
        &Sha256::digest(b"abc")[..],
        decode("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
    assert_eq!(&Sha512::digest(b"abc")[..], decode("ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"));
    let mut h = Sha256::new();
    h.update(b"a");
    h.update(b"bc");
    assert_eq!(h.finalize_reset(), Sha256::digest(b"abc"));
    assert_eq!(h.finalize(), Sha256::digest(b""));
}

#[test]
fn hmac_rfc4231_and_reset() {
    use hmac::{Hmac, HmacReset, KeyInit, Mac, SimpleHmac};
    let key = [0x0b; 20];
    let expected = decode("b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");
    let mut h = Hmac::<Sha256>::new_from_slice(&key).unwrap();
    h.update(b"Hi There");
    h.clone().verify_slice(&expected).unwrap();
    assert!(h.clone().verify_slice(&expected[..31]).is_err());
    let mut wrong = expected.clone();
    wrong[0] ^= 1;
    assert!(h.verify_slice(&wrong).is_err());
    let mut h = HmacReset::<Sha256>::new_from_slice(&key).unwrap();
    for _ in 0..2 {
        h.update(b"Hi There");
        assert_eq!(&h.finalize_reset().into_bytes()[..], expected);
    }
    let mut h = SimpleHmac::<Sha256>::new_from_slice(&key).unwrap();
    h.update(b"Hi There");
    h.verify_slice(&expected).unwrap();
    // RFC 4231 case 6: key exceeds the hash block length.
    let mut h = Hmac::<Sha256>::new_from_slice(&[0xaa; 131]).unwrap();
    h.update(b"Test Using Larger Than Block-Size Key - Hash Key First");
    h.verify_slice(&decode(
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
    ))
    .unwrap();
}

#[test]
fn blake3_known_vector_without_trait_dependency() {
    assert_eq!(
        blake3::hash(b"").as_bytes().as_slice(),
        decode("af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262")
    );
}

#[cfg(feature = "traits-preview")]
#[test]
fn blake3_optional_traits_and_mac() {
    use blake3::traits::cryptographic_core::{Digest, KeyInit, Mac};
    let expected = blake3::hash(b"abc");
    assert_eq!(
        <blake3::Hasher as Digest>::digest(b"abc").as_slice(),
        expected.as_bytes()
    );
    let mut h = <blake3::Hasher as Digest>::new();
    Digest::update(&mut h, b"abc");
    assert_eq!(
        Digest::finalize_reset(&mut h).as_slice(),
        expected.as_bytes()
    );
    assert_eq!(Digest::finalize(h).as_slice(), blake3::hash(b"").as_bytes());
    let key = [7; 32];
    let expected = blake3::keyed_hash(&key, b"message");
    let mut h = <blake3::Hasher as KeyInit>::new_from_slice(&key).unwrap();
    Mac::update(&mut h, b"message");
    Mac::verify_slice(h, expected.as_bytes()).unwrap();
    assert!(<blake3::Hasher as KeyInit>::new_from_slice(&key[..31]).is_err());
}
