#[test]
fn inspect() {
    let mut seen = vec![];
    assert_done(|| list().inspect(|&a| seen.push(a)).collect(), Ok(vec![1, 2, 3]));
    assert_eq!(seen, [1, 2, 3]);
}
