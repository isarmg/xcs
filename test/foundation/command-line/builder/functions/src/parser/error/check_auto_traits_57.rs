#[test]
fn check_auto_traits() {
    fn assert_traits<
        T: Send + Sync + std::panic::RefUnwindSafe + std::panic::UnwindSafe + Unpin,
    >() {
    }
    assert_traits::<MatchesError>();
}
