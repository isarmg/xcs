#[test]
fn max_by() {
    let it = convert(vec![0, 3, 1, -10].into_iter().map(Ok::<i32, ()>));
    assert_eq!(it.max_by(|a, b| Ok(b.cmp(a))), Ok(Some(-10)));
}
