#[test]
fn inspect_err() {
    let mut seen = vec![];
    assert_done(|| err_list().inspect_err(|&a| seen.push(a)).collect::<Vec<_>>(), Err(3));
    assert_eq!(seen, [3]);
}
