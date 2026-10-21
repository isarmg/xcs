#[test]
fn option() {
    assert_eq!(Ok(Some(())), block_on(Some(ok::<(), ()>(())).into_future()));
    assert_eq!(Ok::<_, ()>(None::<()>), block_on(None::<FutureResult<(), ()>>.into_future()));
}
