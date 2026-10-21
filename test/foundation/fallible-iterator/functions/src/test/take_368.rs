#[test]
fn take() {
    let it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<i32, ()>)).take(2);
    assert_eq!(it.collect::<Vec<_>>().unwrap(), [0, 1]);
}
