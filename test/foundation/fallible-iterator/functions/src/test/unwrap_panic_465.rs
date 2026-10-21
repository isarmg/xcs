#[test]
#[should_panic]
fn unwrap_panic() {
    let _ = convert(vec![Ok(0), Err(())].into_iter())
        .unwrap()
        .collect::<Vec<_>>();
}
