
#[cfg(getrandom_backend = "custom")]
#[no_mangle]
unsafe fn __getrandom_v03_custom(dest: *mut u8, len: usize) -> Result<(), getrandom::Error> {
    if len == 13 { return Err(getrandom::Error::UNSUPPORTED); }
    unsafe { core::ptr::write_bytes(dest, 0x6D, len); }
    Ok(())
}
#[cfg(getrandom_backend = "custom")]
#[test]
fn custom_exact_and_error() {
    let mut b = [0; 7];
    getrandom::fill(&mut b).unwrap();
    assert_eq!(b, [0x6D; 7]);
    assert_eq!(getrandom::fill(&mut [0; 13]), Err(getrandom::Error::UNSUPPORTED));
}
