#[test]
fn map() {
    assert_done(|| list().map(|a| a + 1).collect(), Ok(vec![2, 3, 4]));
}
