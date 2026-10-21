#[test]
fn iterator() {
    let it = convert(
        "ab cd"
            .chars()
            .map(|c| if c.is_whitespace() { Err(()) } else { Ok(c) }),
    );

    assert!(it.clone().count().is_err());
    assert!(it.clone().rev().count().is_err());
    assert_eq!(it.clone().iterator().count(), 5);
    assert_eq!(it.clone().iterator().rev().count(), 5);
}
