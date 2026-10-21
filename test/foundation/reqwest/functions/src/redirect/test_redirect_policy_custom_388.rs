#[test]
fn test_redirect_policy_custom() {
    let policy = Policy::custom(|attempt| {
        if attempt.url().host_str() == Some("foo") {
            attempt.stop()
        } else {
            attempt.follow()
        }
    });

    let next = Url::parse("http://bar/baz").unwrap();
    match policy.check(StatusCode::FOUND, &next, &[]) {
        ActionKind::Follow => (),
        other => panic!("unexpected {other:?}"),
    }

    let next = Url::parse("http://foo/baz").unwrap();
    match policy.check(StatusCode::FOUND, &next, &[]) {
        ActionKind::Stop => (),
        other => panic!("unexpected {other:?}"),
    }
}
