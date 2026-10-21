#[test]
fn from_err() {
    assert_done(|| err_list().err_into().collect::<Vec<_>>(), Err(FromErrTest(3)));
}
