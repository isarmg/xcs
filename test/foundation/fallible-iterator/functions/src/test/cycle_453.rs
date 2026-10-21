#[test]
fn cycle() {
    let it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<i32, ()>)).cycle();
    assert_eq!(it.take(6).collect::<Vec<_>>(), Ok(vec![0, 1, 2, 3, 0, 1]));
}
