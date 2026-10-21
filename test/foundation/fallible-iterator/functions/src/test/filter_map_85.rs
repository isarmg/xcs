#[test]
fn filter_map() {
    fn twos_and_threes(x: u32) -> Result<Option<u32>, u32> {
        if x % 2 == 0 {
            Ok(Some(x + 10))
        } else if x % 3 == 0 {
            Ok(None)
        } else {
            Err(x)
        }
    }

    let it = convert(vec![0, 1, 2, 3, 4, 5, 6].into_iter().map(Ok::<u32, u32>))
        .filter_map(twos_and_threes);

    assert_eq!(it.clone().collect::<Vec<_>>(), Err(1));
    assert_eq!(it.rev().collect::<Vec<_>>(), Err(5));

    let it =
        convert(vec![0, 2, 3, 4, 6].into_iter().map(Ok::<u32, u32>)).filter_map(twos_and_threes);

    assert_eq!(it.clone().collect::<Vec<_>>(), Ok(vec![10, 12, 14, 16]));
    assert_eq!(it.rev().collect::<Vec<_>>(), Ok(vec![16, 14, 12, 10]));
}
