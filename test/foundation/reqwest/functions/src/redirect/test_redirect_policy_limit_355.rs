#[test]
fn test_redirect_policy_limit() {
    let policy = Policy::default();
    let next = Url::parse("http://x.y/z").unwrap();
    let mut previous = (0..=9)
        .map(|i| Url::parse(&format!("http://a.b/c/{i}")).unwrap())
        .collect::<Vec<_>>();

    match policy.check(StatusCode::FOUND, &next, &previous) {
        ActionKind::Follow => (),
        other => panic!("unexpected {other:?}"),
    }

    previous.push(Url::parse("http://a.b.d/e/33").unwrap());

    match policy.check(StatusCode::FOUND, &next, &previous) {
        ActionKind::Error(err) if err.is::<TooManyRedirects>() => (),
        other => panic!("unexpected {other:?}"),
    }
}
