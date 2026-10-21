
#[cfg(system_entropy_backend = "custom")]
#[unsafe(no_mangle)]
unsafe fn __system_entropy_custom(dest: *mut u8, len: usize) -> Result<(), system_entropy::Error> {
    if len == 13 { return Err(system_entropy::Error::UNSUPPORTED); }
    unsafe { core::ptr::write_bytes(dest, 0x6D, len); }
    Ok(())
}
#[cfg(system_entropy_backend = "custom")]
#[test]
fn custom_exact_and_error() {
    let mut b = [0; 7];
    system_entropy::fill(&mut b).unwrap();
    assert_eq!(b, [0x6D; 7]);
    assert_eq!(system_entropy::fill(&mut [0; 13]), Err(system_entropy::Error::UNSUPPORTED));
}
