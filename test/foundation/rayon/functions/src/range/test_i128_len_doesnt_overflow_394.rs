#[test]
fn test_i128_len_doesnt_overflow() {
    // Using parse because some versions of rust don't allow long literals
    let octillion: i128 = "1000000000000000000000000000".parse().unwrap();
    let producer = IterProducer {
        range: 0..octillion,
    };

    assert_eq!(octillion as u128, producer.range.unindexed_len());
    assert_eq!(octillion as u128, (0..octillion).unindexed_len());
    assert_eq!(
        2 * octillion as u128,
        (-octillion..octillion).unindexed_len()
    );

    assert_eq!(u128::MAX, (i128::MIN..i128::MAX).unindexed_len());
}
