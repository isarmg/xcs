#[test]
fn unzip() {
    let it = convert(
        vec![(0, 0), (1, -1), (2, -2), (3, -3)]
            .into_iter()
            .map(Ok::<_, ()>),
    );
    let (pos, neg): (Vec<i32>, Vec<i32>) = it.unzip().unwrap();
    assert_eq!(pos, vec![0, 1, 2, 3]);
    assert_eq!(neg, vec![0, -1, -2, -3]);
}
