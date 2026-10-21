#[test]
fn test_short_slice_impl() {
    litemap::testing::check_store::<ShortBoxSlice<(u32, u64)>>();
}
