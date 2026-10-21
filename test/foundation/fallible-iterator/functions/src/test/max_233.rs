#[test]
fn max() {
    let it = convert(vec![0, 3, 1, -10].into_iter().map(Ok::<i32, ()>));
    assert_eq!(it.max().unwrap(), Some(3));
}
