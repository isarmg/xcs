#[test]
fn test_baked_map() {
    test_bake!(
        ZeroMap<str, str>,
        const,
        unsafe {
            #[allow(unused_unsafe)]
            crate::ZeroMap::from_parts_unchecked(
                unsafe {
                    crate::vecs::VarZeroVec16::from_bytes_unchecked(
                        b"\x02\0\0\0\0\0\0\0\x02\0\0\0adbc"
                    )
                },
                unsafe {
                    crate::vecs::VarZeroVec16::from_bytes_unchecked(
                        b"\x02\0\0\0\0\0\0\0\x04\0\0\0ERA1ERA0"
                    )
                },
            )
        },
        zerovec
    );
}
