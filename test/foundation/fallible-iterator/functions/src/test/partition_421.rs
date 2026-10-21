#[test]
fn partition() {
    let it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<i32, ()>));
    let (even, odd): (Vec<i32>, Vec<i32>) = it.partition(|i| Ok(*i % 2 == 0)).unwrap();
    assert_eq!(even, vec![0, 2]);
    assert_eq!(odd, vec![1, 3]);
}
