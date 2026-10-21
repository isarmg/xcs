#[test]
fn inspect() {
    let mut buf = vec![];
    let it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<i32, ()>)).inspect(|&v| {
        buf.push(v);
        Ok(())
    });
    it.count().unwrap();
    assert_eq!(buf, vec![0, 1, 2, 3]);
}
