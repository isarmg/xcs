#[test]
fn peekable() {
    assert_done(|| list().peekable().collect(), Ok(vec![1, 2, 3]));
}
