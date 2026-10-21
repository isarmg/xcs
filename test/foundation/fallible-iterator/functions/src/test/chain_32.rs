#[test]
fn chain() {
    let a = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<u32, ()>));
    let b = convert(vec![4, 5, 6, 7].into_iter().map(Ok::<u32, ()>));
    let it = a.chain(b);

    assert_eq!(it.collect::<Vec<_>>().unwrap(), [0, 1, 2, 3, 4, 5, 6, 7]);

    let a = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<u32, ()>));
    let b = convert(vec![4, 5, 6, 7].into_iter().map(Ok::<u32, ()>));
    let it = a.chain(b).rev();

    assert_eq!(it.collect::<Vec<_>>().unwrap(), [7, 6, 5, 4, 3, 2, 1, 0]);
}
