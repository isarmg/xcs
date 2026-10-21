#[test]
fn skip() {
    assert_done(|| list().skip(2).collect(), Ok(vec![3]));
}
