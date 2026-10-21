#[test]
fn fold() {
    assert_done(|| list().fold(0, |a, b| ok::<i32, u32>(a + b)), Ok(6));
    assert_done(|| err_list().fold(0, |a, b| ok::<i32, u32>(a + b)), Err(3));
}
