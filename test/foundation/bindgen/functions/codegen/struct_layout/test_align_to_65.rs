#[test]
fn test_align_to() {
    assert_eq!(align_to(1, 1), 1);
    assert_eq!(align_to(1, 2), 2);
    assert_eq!(align_to(1, 4), 4);
    assert_eq!(align_to(5, 1), 5);
    assert_eq!(align_to(17, 4), 20);
}
