#[test]
fn or_else() {
    assert_done(|| err_list().or_else(|a| ok::<i32, u32>(a as i32)).collect(), Ok(vec![1, 2, 3]));
}
