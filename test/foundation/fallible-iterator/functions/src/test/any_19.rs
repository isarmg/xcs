#[test]
fn any() {
    assert!(convert([0, 1, 2, 3].iter().map(Ok::<&u32, ()>))
        .any(|&i| Ok(i == 3))
        .unwrap());
    assert!(!convert([0, 1, 2, 4].iter().map(Ok::<&u32, ()>))
        .any(|&i| Ok(i == 3))
        .unwrap());
    assert!(convert([0, 1, 2, 4].iter().map(Ok::<&u32, ()>))
        .any(|_| Err(()))
        .is_err());
}
