#[test]
fn take_while() {
    let it = convert(vec![0, 1, 2, 3, 0].into_iter().map(Ok::<i32, ()>));
    assert_eq!(
        it.clone().take_while(|x| Ok(*x < 0)).collect::<Vec<_>>(),
        Ok(vec![])
    );
    assert_eq!(
        it.clone().take_while(|x| Ok(*x < 2)).collect::<Vec<_>>(),
        Ok(vec![0, 1])
    );
    assert_eq!(
        it.take_while(|x| Ok(*x < 4)).collect::<Vec<_>>(),
        Ok(vec![0, 1, 2, 3, 0])
    );
}
