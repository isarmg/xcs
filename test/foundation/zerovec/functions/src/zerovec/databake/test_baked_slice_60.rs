#[test]
fn test_baked_slice() {
    test_bake!(
        &ZeroSlice<u32>,
        const,
        crate::ZeroSlice::new_empty(),
        zerovec
    );
    test_bake!(
        &ZeroSlice<u32>,
        const,
        unsafe { crate::ZeroSlice::from_bytes_unchecked(b"\x02\x01\0\x16\0M\x01\\") },
        zerovec
    );
}
