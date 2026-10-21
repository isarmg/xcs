#[test]
fn test_count_digits() {
    assert_eq!(count_digits(0), 1);
    assert_eq!(count_digits(1), 1);
    assert_eq!(count_digits(9), 1);
    assert_eq!(count_digits(10), 2);
    assert_eq!(count_digits(99), 2);
    assert_eq!(count_digits(100), 3);
    assert_eq!(count_digits(9999), 4);
    assert_eq!(count_digits(65535), 5);
}
