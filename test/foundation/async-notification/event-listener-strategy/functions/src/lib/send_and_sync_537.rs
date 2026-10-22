#[test]
fn send_and_sync() {
    fn assert_send_and_sync<T: Send + Sync>() {}

    #[cfg(all(feature = "std", not(target_family = "wasm")))]
    {
        assert_send_and_sync::<Blocking>();
        assert_send_and_sync::<Ready>();
    }

    assert_send_and_sync::<NonBlocking<'static>>();
    assert_send_and_sync::<FutureWrapper<()>>();
}
