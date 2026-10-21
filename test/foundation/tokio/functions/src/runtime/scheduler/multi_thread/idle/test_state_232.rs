#[test]
fn test_state() {
    assert_eq!(0, UNPARK_MASK & SEARCH_MASK);
    assert_eq!(0, !(UNPARK_MASK | SEARCH_MASK));

    let state = State::new(10);
    assert_eq!(10, state.num_unparked());
    assert_eq!(0, state.num_searching());
}
