#[test]
fn filter() {
    let it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<u32, u32>));
    let it = it.filter(|&x| if x % 2 == 0 { Ok(x % 3 == 0) } else { Err(x) });

    assert_eq!(it.clone().collect::<Vec<_>>(), Err(1));
    assert_eq!(it.rev().collect::<Vec<_>>(), Err(3));

    let it = convert(vec![0, 2, 4, 6].into_iter().map(Ok::<u32, u32>));
    let it = it.filter(|&x| if x % 2 == 0 { Ok(x % 3 == 0) } else { Err(x) });

    assert_eq!(it.clone().collect::<Vec<_>>(), Ok(vec![0, 6]));
    assert_eq!(it.rev().collect::<Vec<_>>(), Ok(vec![6, 0]))
}
