#[test]
fn enumerate() {
    let it = convert(vec![5, 6, 7, 8].into_iter().map(Ok::<u32, ()>)).enumerate();

    assert_eq!(
        it.collect::<Vec<_>>().unwrap(),
        [(0, 5), (1, 6), (2, 7), (3, 8)]
    );
}
