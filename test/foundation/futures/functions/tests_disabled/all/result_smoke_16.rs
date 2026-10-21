#[test]
fn result_smoke() {
    fn is_future_v<A, B, C>(_: C)
    where
        A: Send + 'static,
        B: Send + 'static,
        C: Future<Item = A, Error = B>,
    {
    }

    is_future_v::<i32, u32, _>(f_ok(1).map(|a| a + 1));
    is_future_v::<i32, u32, _>(f_ok(1).map_err(|a| a + 1));
    is_future_v::<i32, u32, _>(f_ok(1).and_then(Ok));
    is_future_v::<i32, u32, _>(f_ok(1).or_else(Err));
    is_future_v::<(i32, i32), u32, _>(f_ok(1).join(Err(3)));
    is_future_v::<i32, u32, _>(f_ok(1).map(f_ok).flatten());

    assert_done(|| f_ok(1), r_ok(1));
    assert_done(|| f_err(1), r_err(1));
    assert_done(|| result(Ok(1)), r_ok(1));
    assert_done(|| result(Err(1)), r_err(1));
    assert_done(|| ok(1), r_ok(1));
    assert_done(|| err(1), r_err(1));
    assert_done(|| f_ok(1).map(|a| a + 2), r_ok(3));
    assert_done(|| f_err(1).map(|a| a + 2), r_err(1));
    assert_done(|| f_ok(1).map_err(|a| a + 2), r_ok(1));
    assert_done(|| f_err(1).map_err(|a| a + 2), r_err(3));
    assert_done(|| f_ok(1).and_then(|a| Ok(a + 2)), r_ok(3));
    assert_done(|| f_err(1).and_then(|a| Ok(a + 2)), r_err(1));
    assert_done(|| f_ok(1).and_then(|a| Err(a as u32 + 3)), r_err(4));
    assert_done(|| f_err(1).and_then(|a| Err(a as u32 + 4)), r_err(1));
    assert_done(|| f_ok(1).or_else(|a| Ok(a as i32 + 2)), r_ok(1));
    assert_done(|| f_err(1).or_else(|a| Ok(a as i32 + 2)), r_ok(3));
    assert_done(|| f_ok(1).or_else(|a| Err(a + 3)), r_ok(1));
    assert_done(|| f_err(1).or_else(|a| Err(a + 4)), r_err(5));
    assert_done(|| f_ok(1).select(f_err(2)).then(unselect), r_ok(1));
    assert_done(|| f_ok(1).select(Ok(2)).then(unselect), r_ok(1));
    assert_done(|| f_err(1).select(f_ok(1)).then(unselect), r_err(1));
    assert_done(|| f_ok(1).select(empty()).then(unselect), Ok(1));
    assert_done(|| empty().select(f_ok(1)).then(unselect), Ok(1));
    assert_done(|| f_ok(1).join(f_err(1)), Err(1));
    assert_done(|| f_ok(1).join(Ok(2)), Ok((1, 2)));
    assert_done(|| f_err(1).join(f_ok(1)), Err(1));
    assert_done(|| f_ok(1).then(|_| Ok(2)), r_ok(2));
    assert_done(|| f_ok(1).then(|_| Err(2)), r_err(2));
    assert_done(|| f_err(1).then(|_| Ok(2)), r_ok(2));
    assert_done(|| f_err(1).then(|_| Err(2)), r_err(2));
}
