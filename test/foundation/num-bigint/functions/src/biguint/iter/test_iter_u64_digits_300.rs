#[test]
fn test_iter_u64_digits() {
    let n = super::BigUint::from(5u8);
    let mut it = n.iter_u64_digits();
    assert_eq!(it.len(), 1);
    assert_eq!(it.next(), Some(5));
    assert_eq!(it.len(), 0);
    assert_eq!(it.next(), None);
    assert_eq!(it.len(), 0);
    assert_eq!(it.next(), None);

    let n = super::BigUint::from(18_446_744_073_709_551_616u128);
    let mut it = n.iter_u64_digits();
    assert_eq!(it.len(), 2);
    assert_eq!(it.next(), Some(0));
    assert_eq!(it.len(), 1);
    assert_eq!(it.next(), Some(1));
    assert_eq!(it.len(), 0);
    assert_eq!(it.next(), None);
}
