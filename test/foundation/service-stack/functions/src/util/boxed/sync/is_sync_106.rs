#[test]
fn is_sync() {
    fn assert_sync<T: Sync>() {}

    assert_sync::<BoxService<(), (), ()>>();
}
