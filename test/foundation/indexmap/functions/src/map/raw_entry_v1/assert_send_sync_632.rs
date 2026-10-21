#[test]
fn assert_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RawEntryMut<'_, i32, i32, ()>>();
}
