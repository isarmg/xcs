#[test]
fn test_baked_vec() {
    test_bake!(
        VarZeroVec<str>,
        const,
        crate::vecs::VarZeroVec16::new(),
        zerovec
    );

    test_bake!(
        VarZeroVec<str>,
        const,
        unsafe {
            crate::vecs::VarZeroVec16::from_bytes_unchecked(b"\x02\0\0\0\0\0\x05\0helloworld")
        },
        zerovec
    );
}
