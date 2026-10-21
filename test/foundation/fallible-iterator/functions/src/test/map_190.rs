#[test]
fn map() {
    let it = convert(vec![0, 1, 2, 3, 4].into_iter().map(Ok::<u32, ()>)).map(|n| Ok(n * 2));
    fn assert_debug(_: &impl core::fmt::Debug) {}
    assert_debug(&it);
    assert_eq!(it.clone().collect::<Vec<_>>().unwrap(), [0, 2, 4, 6, 8]);
    assert_eq!(it.rev().collect::<Vec<_>>().unwrap(), [8, 6, 4, 2, 0]);

    let it = convert(vec![0, 1, 2, 3, 4].into_iter().map(Ok::<u32, ()>)).map(|n| {
        if n == 2 {
            Err(())
        } else {
            Ok(n * 2)
        }
    });

    {
        let mut it = it.clone();
        assert_eq!(it.next(), Ok(Some(0)));
        assert_eq!(it.next(), Ok(Some(2)));
        assert_eq!(it.next(), Err(()));
    }

    {
        let mut it = it.rev();
        assert_eq!(it.next(), Ok(Some(8)));
        assert_eq!(it.next(), Ok(Some(6)));
        assert_eq!(it.next(), Err(()));
    }
}
