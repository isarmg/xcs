#[test]
fn take() {
    assert_done(|| list().take(2).collect(), Ok(vec![1, 2]));
}
