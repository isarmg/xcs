#[test]
fn iter() {
    let x = 3;
    let mut iter = match x {
        3 => Left(0..10),
        _ => Right(17..),
    };

    assert_eq!(iter.next(), Some(0));
    assert_eq!(iter.count(), 9);
}
