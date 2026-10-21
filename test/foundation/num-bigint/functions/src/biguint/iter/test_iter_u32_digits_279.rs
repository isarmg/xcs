#[test]
fn test_iter_u32_digits() {
    let n = super::BigUint::from(5u8);
    let mut it = n.iter_u32_digits();
    assert_eq!(it.len(), 1);
    assert_eq!(it.next(), Some(5));
    assert_eq!(it.len(), 0);
    assert_eq!(it.next(), None);
    assert_eq!(it.len(), 0);
    assert_eq!(it.next(), None);

    let n = super::BigUint::from(112500000000u64);
    let mut it = n.iter_u32_digits();
    assert_eq!(it.len(), 2);
    assert_eq!(it.next(), Some(830850304));
    assert_eq!(it.len(), 1);
    assert_eq!(it.next(), Some(26));
    assert_eq!(it.len(), 0);
    assert_eq!(it.next(), None);
}
