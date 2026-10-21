#[cfg(not(any(crossbeam_sanitize, miri)))] // `crossbeam_sanitize` and `miri` reduce the size of `Local`
#[test]
fn local_size() {
    // TODO: https://github.com/crossbeam-rs/crossbeam/issues/869
    // assert!(
    //     core::mem::size_of::<Local>() <= 2048,
    //     "An allocation of `Local` should be <= 2048 bytes."
    // );
}
