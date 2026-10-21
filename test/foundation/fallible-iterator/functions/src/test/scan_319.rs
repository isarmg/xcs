#[test]
fn scan() {
    let it = convert(vec![1, 2, 3, 4].into_iter().map(Ok::<i32, ()>)).scan(0, |st, v| {
        if v > 3 {
            Ok(None)
        } else {
            *st += v;
            Ok(Some(-*st))
        }
    });
    assert_eq!(it.collect::<Vec<_>>(), Ok(vec![-1, -3, -6]));
}
