#[test]
fn last() {
    let it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<u32, ()>));
    assert_eq!(it.last().unwrap(), Some(3));
}
