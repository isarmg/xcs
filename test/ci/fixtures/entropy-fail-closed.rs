//! Maintained consumer regression: entropy failure must never become hash seeding fallback.
use core::mem::MaybeUninit;

#[test]
fn nonempty_requests_fail_and_empty_requests_preserve_their_contract() {
    let mut bytes = [0x5a; 32];
    assert_eq!(system_entropy::fill(&mut bytes), Err(system_entropy::Error::UNSUPPORTED));
    assert_eq!(bytes, [0x5a; 32]);
    let mut uninit = [MaybeUninit::uninit(); 32];
    assert!(matches!(system_entropy::fill_uninit(&mut uninit), Err(system_entropy::Error::UNSUPPORTED)));
    assert_eq!(system_entropy::u32(), Err(system_entropy::Error::UNSUPPORTED));
    assert_eq!(system_entropy::u64(), Err(system_entropy::Error::UNSUPPORTED));
    assert!(system_entropy::fill(&mut []).is_ok());
    assert_eq!(system_entropy::fill_uninit(&mut []).unwrap().len(), 0);
}

#[test]
#[should_panic(expected = "system_entropy::fill() failed.")]
fn default_ahash_seeding_does_not_substitute_a_noncryptographic_rng() {
    let _ = ahash::RandomState::new();
}
