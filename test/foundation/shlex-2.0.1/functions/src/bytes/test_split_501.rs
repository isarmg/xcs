#[test]
fn test_split() {
    for &(input, output) in SPLIT_TEST_ITEMS {
        assert_eq!(split(input), output.map(|o| o.iter().map(|&x| x.to_owned()).collect()));
    }
}
