#[test]
fn test_bytes_from_bits_pow2() {
    assert_eq!(bytes_from_bits_pow2(0), 0);
    for i in 1..9 {
        assert_eq!(bytes_from_bits_pow2(i), 1);
    }
    for i in 9..17 {
        assert_eq!(bytes_from_bits_pow2(i), 2);
    }
    for i in 17..33 {
        assert_eq!(bytes_from_bits_pow2(i), 4);
    }
}
