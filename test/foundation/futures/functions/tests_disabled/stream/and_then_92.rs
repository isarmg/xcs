#[test]
fn and_then() {
    assert_done(|| list().and_then(|a| Ok(a + 1)).collect(), Ok(vec![2, 3, 4]));
    assert_done(|| list().and_then(|a| err::<i32, u32>(a as u32)).collect::<Vec<_>>(), Err(1));
}
