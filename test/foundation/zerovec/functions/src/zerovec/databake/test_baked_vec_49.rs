#[test]
fn test_baked_vec() {
    test_bake!(ZeroVec<u32>, const, crate::ZeroVec::new(), zerovec);
    test_bake!(
        ZeroVec<u32>,
        const,
        unsafe { crate::ZeroVec::from_bytes_unchecked(b"\x02\x01\0\x16\0M\x01\\") },
        zerovec
    );
}
