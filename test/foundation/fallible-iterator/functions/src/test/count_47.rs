#[test]
fn count() {
    assert_eq!(
        convert([0, 1, 2, 3].iter().map(Ok::<&u32, ()>))
            .count()
            .unwrap(),
        4
    );

    let it = Some(Ok(1)).into_iter().chain(iter::repeat(Err(())));
    assert!(convert(it).count().is_err());
}
