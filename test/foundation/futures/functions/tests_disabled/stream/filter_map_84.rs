#[test]
fn filter_map() {
    assert_done(
        || list().filter_map(|x| ok(if x % 2 == 0 { Some(x + 10) } else { None })).collect(),
        Ok(vec![12]),
    );
}
