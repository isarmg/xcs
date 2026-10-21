#[cfg(feature = "dev")]
#[test]
fn supported_dev_helpers_validate_fixed_hash() {
    let vector = cryptographic_core::dev::TestVector {
        input: b"abc",
        output: b"abc\0\0\0\0\0",
    };
    assert_eq!(
        cryptographic_core::dev::fixed_test::<FixedHashWithSer>(&vector),
        Ok(())
    );
    assert_eq!(
        cryptographic_core::dev::fixed_reset_test::<FixedHashWithSer>(&vector),
        Ok(())
    );
}
