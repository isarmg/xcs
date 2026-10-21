#[test]
fn map_err() {
    let it = convert(
        vec![0, 1, 2, 3]
            .into_iter()
            .map(|n| if n % 2 == 0 { Ok(n) } else { Err(n) }),
    );

    assert_eq!(it.clone().collect::<Vec<_>>(), Err(1));
    assert_eq!(it.rev().collect::<Vec<_>>(), Err(3));
}
