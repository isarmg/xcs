#[test]
fn min_by() {
    let it = convert(vec![0, 3, 1, -10].into_iter().map(Ok::<i32, ()>));
    assert_eq!(it.min_by(|a, b| Ok(b.cmp(a))), Ok(Some(3)));
}
