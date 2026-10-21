#[test]
fn test_ok() {
    assert_done(|| ok(1), r_ok(1));
    assert_done(|| err(1), r_err(1));
}
