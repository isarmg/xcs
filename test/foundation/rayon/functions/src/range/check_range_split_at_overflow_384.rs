#[test]
fn check_range_split_at_overflow() {
    // Note, this split index overflows i8!
    let producer = IterProducer { range: -100i8..100 };
    let (left, right) = producer.split_at(150);
    let r1: i32 = left.range.map(i32::from).sum();
    let r2: i32 = right.range.map(i32::from).sum();
    assert_eq!(r1 + r2, -100);
}
