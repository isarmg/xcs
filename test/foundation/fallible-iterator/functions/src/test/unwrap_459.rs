#[test]
fn unwrap() {
    let it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<i32, ()>)).unwrap();
    assert_eq!(it.collect::<Vec<_>>(), vec![0, 1, 2, 3]);
}
