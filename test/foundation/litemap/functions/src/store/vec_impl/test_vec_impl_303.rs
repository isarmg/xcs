#[test]
fn test_vec_impl() {
    crate::testing::check_store_full::<Vec<(u32, u64)>>();
}
