//! Guard against substring migrations changing block_api into bsynchronization::api.
//! Exercise exported macro expansion in a downstream crate, not a toy hash core.
use cryptographic_core::{block_api::CtOutWrapper, consts::U32, Digest};

cryptographic_core::buffer_fixed!(
    pub struct WrappedSha256(CtOutWrapper<sha2::block_api::Sha256VarCore, U32>);
    impl: BaseFixedTraits AlgorithmName Default Clone HashMarker Reset FixedOutputReset SerializableState;
);

fn decode(value: &str) -> Vec<u8> {
    value.as_bytes().chunks_exact(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn exported_block_api_macro_and_sha2_vector() {
    // Same independent SHA-256 vector as crypto-consumers.rs.
    let expected = decode("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(&WrappedSha256::digest(b"abc")[..], expected);
    assert_eq!(&sha2::Sha256::digest(b"abc")[..], expected);
    let mut h = WrappedSha256::new();
    h.update(b"a");
    h.update(b"bc");
    assert_eq!(&h.finalize_reset()[..], expected);
    assert_eq!(h.finalize(), sha2::Sha256::digest(b""));
}

#[test]
fn hmac_rfc4231_vector() {
    use hmac::{Hmac, KeyInit, Mac};
    let mut h = Hmac::<WrappedSha256>::new_from_slice(&[0x0b; 20]).unwrap();
    Mac::update(&mut h, b"Hi There");
    h.verify_slice(&decode(
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
    )).unwrap();
}

#[test]
fn blake2_fixed_and_variable_vectors() {
    use blake2::{Blake2b512, Blake2bVar, Blake2s256, VariableOutput};
    let expected = decode("ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923");
    assert_eq!(&Blake2b512::digest(b"abc")[..], expected);
    let mut out = [0; 64];
    Blake2bVar::digest_variable(b"abc", &mut out).unwrap();
    assert_eq!(out.as_slice(), expected);
    assert_eq!(&Blake2s256::digest(b"abc")[..], decode(
        "508c5e8c327c14e2e1a72ba34eeb452f37458b209ed63a294d999b4c86675982",
    ));
}
