#[test]
fn zip() {
    assert_done(|| list().zip(list()).collect(), Ok(vec![(1, 1), (2, 2), (3, 3)]));
    assert_done(|| list().zip(list().take(2)).collect(), Ok(vec![(1, 1), (2, 2)]));
    assert_done(|| list().take(2).zip(list()).collect(), Ok(vec![(1, 1), (2, 2)]));
    assert_done(|| err_list().zip(list()).collect::<Vec<_>>(), Err(3));
    assert_done(|| list().zip(list().map(|x| x + 1)).collect(), Ok(vec![(1, 2), (2, 3), (3, 4)]));
}
