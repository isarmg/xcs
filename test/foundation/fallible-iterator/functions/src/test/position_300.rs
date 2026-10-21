#[test]
fn position() {
    let mut it = convert(vec![1, 2, 3, 4].into_iter().map(Ok::<i32, ()>));
    assert_eq!(it.position(|n| Ok(n == 2)).unwrap(), Some(1));
    assert_eq!(it.position(|n| Ok(n == 3)).unwrap(), Some(0));
    assert_eq!(it.position(|n| Ok(n == 5)).unwrap(), None);

    let mut it = convert(vec![1, 2, 3, 4].into_iter().map(Ok::<i32, i32>));
    assert_eq!(
        it.clone()
            .position(|n| if n == 3 { Err(42) } else { Ok(n == 2) }),
        Ok(Some(1))
    );
    assert_eq!(
        it.position(|n| if n == 3 { Err(42) } else { Ok(n == 4) }),
        Err(42)
    );
}
