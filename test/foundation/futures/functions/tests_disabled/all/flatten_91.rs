#[test]
fn flatten() {
    fn ok<T: Send + 'static>(a: T) -> FutureResult<T, u32> {
        future::ok(a)
    }
    fn err<E: Send + 'static>(b: E) -> FutureResult<i32, E> {
        future::err(b)
    }

    assert_done(|| ok(ok(1)).flatten(), r_ok(1));
    assert_done(|| ok(err(1)).flatten(), r_err(1));
    assert_done(|| err(1u32).map(ok).flatten(), r_err(1));
    assert_done(|| future::ok(future::ok(1)).flatten(), r_ok(1));
    assert_empty(|| ok(empty::<i32, u32>()).flatten());
    assert_empty(|| empty::<i32, u32>().map(ok).flatten());
}
