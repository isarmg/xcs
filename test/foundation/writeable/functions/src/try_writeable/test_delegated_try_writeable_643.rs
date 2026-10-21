#[test]
fn test_delegated_try_writeable() {
    let mut message = DelegatedTryMessage(Ok("success"));
    assert_try_writeable_eq!(message, "success");
    message = DelegatedTryMessage(Err(44));
    assert_try_writeable_eq!(message, "44", Err(44));
}
