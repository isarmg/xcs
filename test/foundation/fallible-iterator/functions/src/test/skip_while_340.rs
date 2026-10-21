#[test]
fn skip_while() {
    let it = convert(vec![1, 2, 3, 4, 1].into_iter().map(Ok::<i32, ()>));
    assert_eq!(
        it.clone().skip_while(|x| Ok(*x < 1)).collect::<Vec<_>>(),
        Ok(vec![1, 2, 3, 4, 1])
    );
    assert_eq!(
        it.clone().skip_while(|x| Ok(*x < 3)).collect::<Vec<_>>(),
        Ok(vec![3, 4, 1])
    );
    assert_eq!(
        it.skip_while(|x| Ok(*x < 5)).collect::<Vec<_>>(),
        Ok(vec![])
    );
}
