#[test]
fn forward() {
    let v = Vec::new();
    let v = block_on(iter_ok::<_, Never>(vec![0, 1]).forward(v)).unwrap().1;
    assert_eq!(v, vec![0, 1]);

    let v = block_on(iter_ok::<_, Never>(vec![2, 3]).forward(v)).unwrap().1;
    assert_eq!(v, vec![0, 1, 2, 3]);

    assert_done(
        move || iter_ok::<_, Never>(vec![4, 5]).forward(v).map(|(_, s)| s),
        Ok(vec![0, 1, 2, 3, 4, 5]),
    );
}
