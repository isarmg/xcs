#[test]
fn test_empty() {
    fn empty() -> Empty<i32, u32> {
        future::empty()
    }

    assert_empty(|| empty());
    assert_empty(|| empty().select(empty()));
    assert_empty(|| empty().join(empty()));
    assert_empty(|| empty().join(f_ok(1)));
    assert_empty(|| f_ok(1).join(empty()));
    assert_empty(|| empty().or_else(move |_| empty()));
    assert_empty(|| empty().and_then(move |_| empty()));
    assert_empty(|| f_err(1).or_else(move |_| empty()));
    assert_empty(|| f_ok(1).and_then(move |_| empty()));
    assert_empty(|| empty().map(|a| a + 1));
    assert_empty(|| empty().map_err(|a| a + 1));
    assert_empty(|| empty().then(|a| a));
}
