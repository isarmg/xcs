#[test]
fn test_construction() {
    let mut rng = OsRng::default();
    assert!(rng.next_u64() != 0);
}
