#[test]
fn find() {
    let mut it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<u32, u32>));

    assert_eq!(it.find(|x| Ok(x % 2 == 1)), Ok(Some(1)));
    assert_eq!(it.next(), Ok(Some(2)));

    let it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<u32, u32>));
    assert_eq!(
        it.clone()
            .find(|&x| if x == 2 { Err(29) } else { Ok(false) }),
        Err(29)
    );
    assert_eq!(
        it.clone()
            .find(|&x| if x == 2 { Err(29) } else { Ok(true) }),
        Ok(Some(0))
    );
    assert_eq!(
        it.clone()
            .rev()
            .find(|&x| if x == 2 { Err(29) } else { Ok(false) }),
        Err(29)
    );
    assert_eq!(
        it.rev().find(|&x| if x == 2 { Err(29) } else { Ok(true) }),
        Ok(Some(3))
    );
}
