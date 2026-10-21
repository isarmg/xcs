#[test]
fn flatten() {
    let it = convert(
        vec![0..1, 0..0, 1..5]
            .into_iter()
            .map(|r| convert(r.map(Ok::<i32, ()>)))
            .map(Ok::<_, ()>),
    )
    .flatten();
    assert_eq!(it.collect::<Vec<_>>(), Ok(vec![0, 1, 2, 3, 4]));
}
