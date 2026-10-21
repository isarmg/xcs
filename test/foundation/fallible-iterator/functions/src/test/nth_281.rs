#[test]
fn nth() {
    let mut it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<i32, ()>));
    assert_eq!(it.nth(1).unwrap(), Some(1));
    assert_eq!(it.nth(0).unwrap(), Some(2));
    assert_eq!(it.nth(2).unwrap(), None);
}
