#[test]
fn test_baked_slice() {
    test_bake!(
        &VarZeroSlice<str>,
        const,
        crate::vecs::VarZeroSlice16::new_empty(),
        zerovec
    );
    test_bake!(
        &VarZeroSlice<str>,
        const,
        unsafe {
            crate::vecs::VarZeroSlice16::from_bytes_unchecked(b"\x02\0\0\0\0\0\x05\0helloworld")
        },
        zerovec
    );
}
