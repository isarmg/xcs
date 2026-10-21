#[test]
fn flatten() {
    assert_done(|| list().map(|_| list()).flatten().collect(), Ok(vec![1, 2, 3, 1, 2, 3, 1, 2, 3]));
}
