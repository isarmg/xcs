#[test]
fn check_auto_traits() {
    fn assert_traits<T: Send + Sync + Unpin>() {}
    assert_traits::<Error>();
}
