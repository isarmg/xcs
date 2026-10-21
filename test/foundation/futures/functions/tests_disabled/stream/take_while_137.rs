#[test]
fn take_while() {
    assert_done(|| list().take_while(|e| Ok(*e < 3)).collect(), Ok(vec![1, 2]));
}
