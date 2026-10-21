#[test]
fn test_fallible() {
    assert_eq!(try_join(vec!["\0"]), Err(QuoteError::Nul));
    assert_eq!(try_quote("\0"), Err(QuoteError::Nul));
}
