#[test]
fn flat_map() {
    let it = convert(vec![0..1, 0..0, 1..5].into_iter().map(Ok::<Range<i32>, ()>))
        .flat_map(|r| Ok(convert(r.map(Ok::<i32, ()>))));
    assert_eq!(it.collect::<Vec<_>>(), Ok(vec![0, 1, 2, 3, 4]));
}
