#[test]
fn wait() {
    assert_eq!(block_on_stream(list()).collect::<Result<Vec<_>, _>>(), Ok(vec![1, 2, 3]));
}
