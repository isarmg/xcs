use blake2::{
    digest::{Digest, FixedOutput, KeyInit, Update},
    Blake2b512, Blake2bVar, Blake2s256, Blake2sVar, VariableOutput,
};
#[test]
fn runtime_lengths_and_errors() {
    for n in [0, 1, 32, 64] {
        let h = Blake2bVar::new(n).unwrap();
        assert_eq!(h.output_size(), n);
        assert_eq!(
            h.clone()
                .finalize_variable(&mut vec![0; n + 1])
                .unwrap_err()
                .to_string(),
            "invalid buffer length"
        );
        let mut out = vec![0; n];
        h.finalize_variable(&mut out).unwrap();
    }
    for n in [0, 1, 16, 32] {
        let h = Blake2sVar::new(n).unwrap();
        assert_eq!(h.output_size(), n);
        let mut out = vec![0; n];
        h.finalize_variable(&mut out).unwrap();
    }
    assert!(Blake2bVar::new(65).is_err());
    assert!(Blake2sVar::new(33).is_err());
    assert!(blake2::Blake2bMac512::new_from_slice(&[0; 65]).is_err());
    assert!(blake2::Blake2sMac256::new_from_slice(&[0; 33]).is_err());
}

// Exhaust every length through the former block-sized acceptance range, plus
// one above it. A panic fails the test: this Result API must return InvalidLength.
#[test]
fn personalized_mac_parameter_boundaries() {
    macro_rules! check {
        ($mac:ty, $maximum:expr) => {{
            let max_key = $maximum;
            let max_parameter = max_key / 4;
            for length in 0..=2 * max_key + 1 {
                let bytes = vec![0x42; length];
                assert_eq!(
                    <$mac>::new_with_salt_and_personal(&bytes, &[], &[]).is_ok(),
                    length <= max_key,
                    "key length {length}"
                );
                assert_eq!(
                    <$mac>::new_with_salt_and_personal(&[1], &bytes, &[]).is_ok(),
                    length <= max_parameter,
                    "salt length {length}"
                );
                assert_eq!(
                    <$mac>::new_with_salt_and_personal(&[1], &[], &bytes).is_ok(),
                    length <= max_parameter,
                    "persona length {length}"
                );
            }
            // Simultaneous exact maxima are valid, independently of output size.
            let key = vec![1; max_key];
            let parameter = vec![2; max_parameter];
            assert!(<$mac>::new_with_salt_and_personal(&key, &parameter, &parameter).is_ok());
            assert!(<$mac>::new_with_salt_and_personal(
                &vec![1; max_key + 1],
                &vec![2; max_parameter + 1],
                &vec![3; max_parameter + 1]
            )
            .is_err());
        }};
    }
    check!(blake2::Blake2bMac512, 64);
    check!(blake2::Blake2sMac256, 32);
    check!(blake2::Blake2bMac<blake2::digest::consts::U16>, 64);
    check!(blake2::Blake2sMac<blake2::digest::consts::U16>, 32);
}

#[test]
fn fixed_types() {
    let data = [42u8; 257];
    let b = Blake2b512::digest(data);
    let s = Blake2s256::digest(data);
    let mut vb = [0; 64];
    let mut vs = [0; 32];
    Blake2bVar::digest_variable(data, &mut vb).unwrap();
    Blake2sVar::digest_variable(data, &mut vs).unwrap();
    assert_eq!(&b[..], vb);
    assert_eq!(&s[..], vs);
}
#[cfg(feature = "std")]
#[test]
fn io_write() {
    use std::io::Write;
    let mut h = Blake2b512::new();
    h.write_all(b"hello").unwrap();
    h.flush().unwrap();
    assert_eq!(h.finalize(), Blake2b512::digest(b"hello"));
    let mut h = Blake2bVar::new(17).unwrap();
    h.write_all(b"hello").unwrap();
    h.flush().unwrap();
    let mut x = [0; 17];
    h.finalize_variable(&mut x).unwrap();
    let mut y = [0; 17];
    Blake2bVar::digest_variable(b"hello", &mut y).unwrap();
    assert_eq!(x, y);
}
#[cfg(feature = "alloc")]
#[test]
fn boxed_runtime_output() {
    let mut h = Blake2bVar::new(17).unwrap();
    h.update(b"hello");
    let x = h.finalize_boxed();
    let mut y = [0; 17];
    Blake2bVar::digest_variable(b"hello", &mut y).unwrap();
    assert_eq!(&x[..], y);
}
#[cfg(feature = "reset")]
#[test]
fn reset_preserves_configuration() {
    use blake2::{
        digest::{FixedOutputReset, Reset},
        VariableOutputReset,
    };
    for n in [0, 1, 16, 32, 64] {
        let mut h = Blake2bVar::new(n).unwrap();
        h.update(b"hello");
        let mut expected = vec![0; n];
        h.clone().finalize_variable(&mut expected).unwrap();
        assert!(h.finalize_variable_reset(&mut vec![0; n + 1]).is_err());
        let mut got = vec![0; n];
        h.finalize_variable_reset(&mut got).unwrap();
        assert_eq!(got, expected);
        h.update(b"hello");
        h.finalize_variable_reset(&mut got).unwrap();
        assert_eq!(got, expected);
        h.update(b"junk");
        h.reset();
        h.update(b"hello");
        h.finalize_variable_reset(&mut got).unwrap();
        assert_eq!(got, expected);
    }
    let mut h = Blake2b512::new();
    Digest::update(&mut h, b"hello");
    assert_eq!(h.finalize_fixed_reset(), Blake2b512::digest(b"hello"));
    Digest::update(&mut h, b"hello");
    assert_eq!(h.finalize_fixed_reset(), Blake2b512::digest(b"hello"));
    for key in [vec![7], vec![3; 32], vec![5; 64]] {
        let mut h = blake2::Blake2bMac512::new_from_slice(&key).unwrap();
        h.update(b"hello");
        let expected = h.clone().finalize_fixed();
        assert_eq!(h.finalize_fixed_reset(), expected);
        h.update(b"hello");
        assert_eq!(h.finalize_fixed_reset(), expected);
    }
}
#[test]
fn constant_time_mac_verification_interface() {
    use blake2::digest::Mac;
    let mut h = blake2::Blake2bMac512::new_from_slice(b"secret").unwrap();
    Update::update(&mut h, b"message");
    let tag = h.clone().finalize_fixed();
    assert!(h.clone().verify_slice(&tag).is_ok());
    let mut wrong = tag;
    wrong[0] ^= 1;
    assert!(h.clone().verify_slice(&wrong).is_err());
    assert!(h.verify_slice(&wrong[..63]).is_err());
}
#[cfg(feature = "reset")]
#[test]
fn small_reset_and_personalized_reset() {
    use blake2::{digest::FixedOutputReset, VariableOutputReset};
    let mut h = Blake2sVar::new(13).unwrap();
    h.update(b"hello");
    let mut first = [0; 13];
    h.finalize_variable_reset(&mut first).unwrap();
    h.update(b"hello");
    let mut second = [0; 13];
    h.finalize_variable_reset(&mut second).unwrap();
    assert_eq!(first, second);
    for key in [vec![2], vec![4; 32]] {
        let mut m =
            blake2::Blake2sMac256::new_with_salt_and_personal(&key, b"salt", b"personal").unwrap();
        m.update(b"hello");
        let first = m.finalize_fixed_reset();
        m.update(b"hello");
        assert_eq!(first, m.finalize_fixed_reset());
    }
}
