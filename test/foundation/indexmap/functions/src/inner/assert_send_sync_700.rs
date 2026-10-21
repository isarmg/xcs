#[test]
fn assert_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Core<i32, i32>>();
}
