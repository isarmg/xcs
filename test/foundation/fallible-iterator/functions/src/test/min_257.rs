#[test]
fn min() {
    let it = convert(vec![0, 3, -10, 1].into_iter().map(Ok::<i32, ()>));
    assert_eq!(it.min().unwrap(), Some(-10));
}
