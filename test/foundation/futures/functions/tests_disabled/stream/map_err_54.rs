#[test]
fn map_err() {
    assert_done(|| err_list().map_err(|a| a + 1).collect::<Vec<_>>(), Err(4));
}
