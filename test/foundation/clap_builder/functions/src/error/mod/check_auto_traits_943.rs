#[test]
fn check_auto_traits() {
    static_assertions::assert_impl_all!(Error: Send, Sync, Unpin);
}
