//! Maintained consumer regression: entropy failure must never become hash seeding fallback.
use core::mem::MaybeUninit;

#[test]
fn nonempty_requests_fail_and_empty_requests_preserve_their_contract() {
    let mut bytes = [0x5a; 32];
    assert_eq!(getrandom::fill(&mut bytes), Err(getrandom::Error::UNSUPPORTED));
    assert_eq!(bytes, [0x5a; 32]);
    let mut uninit = [MaybeUninit::uninit(); 32];
    assert!(matches!(getrandom::fill_uninit(&mut uninit), Err(getrandom::Error::UNSUPPORTED)));
    assert_eq!(getrandom::u32(), Err(getrandom::Error::UNSUPPORTED));
    assert_eq!(getrandom::u64(), Err(getrandom::Error::UNSUPPORTED));
    assert!(getrandom::fill(&mut []).is_ok());
    assert_eq!(getrandom::fill_uninit(&mut []).unwrap().len(), 0);
}

#[test]
#[should_panic(expected = "getrandom::fill() failed.")]
fn default_ahash_seeding_does_not_substitute_a_noncryptographic_rng() {
    let _ = ahash::RandomState::new();
}
