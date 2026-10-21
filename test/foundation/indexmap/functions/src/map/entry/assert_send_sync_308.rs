#[test]
fn assert_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Entry<'_, i32, i32>>();
    assert_send_sync::<IndexedEntry<'_, i32, i32>>();
}
