#[test]
fn then() {
    assert_done(|| list().then(|a| a.map(|e| e + 1)).collect(), Ok(vec![2, 3, 4]));
}
