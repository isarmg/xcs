#[test]
fn filter() {
    assert_done(|| list().filter(|a| ok(*a % 2 == 0)).collect(), Ok(vec![2]));
}
