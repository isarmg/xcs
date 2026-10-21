#[test]
fn test_short_slice_impl_full() {
    litemap::testing::check_store_full::<ShortBoxSlice<(u32, u64)>>();
}
