#[test]
fn test_redirect_policy_limit_to_0() {
    let policy = Policy::limited(0);
    let next = Url::parse("http://x.y/z").unwrap();
    let previous = vec![Url::parse("http://a.b/c").unwrap()];

    match policy.check(StatusCode::FOUND, &next, &previous) {
        ActionKind::Error(err) if err.is::<TooManyRedirects>() => (),
        other => panic!("unexpected {other:?}"),
    }
}
