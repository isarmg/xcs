#[test]
fn concat2() {
    let a = iter_ok::<_, ()>(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    assert_done(move || a.concat(), Ok(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]));

    let b = iter(vec![Ok::<_, ()>(vec![1, 2, 3]), Err(()), Ok(vec![7, 8, 9])]);
    assert_done(move || b.concat(), Err(()));

    let c = empty::<Vec<()>, ()>();
    assert_done(move || c.concat(), Ok(vec![]))
}
