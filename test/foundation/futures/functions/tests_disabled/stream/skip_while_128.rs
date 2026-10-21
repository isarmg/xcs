#[test]
fn skip_while() {
    assert_done(|| list().skip_while(|e| Ok(*e % 2 == 1)).collect(), Ok(vec![2, 3]));
}
