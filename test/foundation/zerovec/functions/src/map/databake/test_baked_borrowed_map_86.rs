#[test]
fn test_baked_borrowed_map() {
    test_bake!(
        ZeroMapBorrowed<str, str>,
        const,
        unsafe {
            #[allow(unused_unsafe)]
            crate::maps::ZeroMapBorrowed::from_parts_unchecked(
                unsafe {
                    crate::vecs::VarZeroSlice16::from_bytes_unchecked(
                        b"\x02\0\0\0\0\0\0\0\x02\0\0\0adbc"
                    )
                },
                unsafe {
                    crate::vecs::VarZeroSlice16::from_bytes_unchecked(
                        b"\x02\0\0\0\0\0\0\0\x04\0\0\0ERA1ERA0"
                    )
                },
            )
        },
        zerovec
    );
}
