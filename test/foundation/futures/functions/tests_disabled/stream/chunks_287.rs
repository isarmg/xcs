#[test]
fn chunks() {
    assert_done(|| list().chunks(3).collect(), Ok(vec![vec![1, 2, 3]]));
    assert_done(|| list().chunks(1).collect(), Ok(vec![vec![1], vec![2], vec![3]]));
    assert_done(|| list().chunks(2).collect(), Ok(vec![vec![1, 2], vec![3]]));
    let mut list = block_on_stream(err_list().chunks(3));
    let i = list.next().unwrap().unwrap();
    assert_eq!(i, vec![1, 2]);
    let i = list.next().unwrap().unwrap_err();
    assert_eq!(i, 3);
}
