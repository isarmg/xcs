#[test]
fn test_os_rng() {
    let x = OsRng.next_u64();
    let y = OsRng.next_u64();
    assert!(x != 0);
    assert!(x != y);
}
