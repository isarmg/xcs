#[test]
fn step_by() {
    let it = convert(
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8]
            .into_iter()
            .map(Ok::<i32, ()>),
    )
    .step_by(3);
    assert_eq!(it.collect::<Vec<_>>(), Ok(vec![0, 3, 6]));
}
