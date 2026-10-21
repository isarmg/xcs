#[test]
fn check_auto_traits() {
    static_assertions::assert_impl_all!(
        MatchesError: Send,
        Sync,
        std::panic::RefUnwindSafe,
        std::panic::UnwindSafe,
        Unpin
    );
}
