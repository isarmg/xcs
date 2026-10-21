#[test]
fn slice_chunk_by_mut() {
    let mut v: Vec<_> = (0..1000).collect();
    assert_eq!(v[..0].par_chunk_by_mut(|_, _| todo!()).count(), 0);
    assert_eq!(v[..1].par_chunk_by_mut(|_, _| todo!()).count(), 1);
    assert_eq!(v[..2].par_chunk_by_mut(|_, _| true).count(), 1);
    assert_eq!(v[..2].par_chunk_by_mut(|_, _| false).count(), 2);

    let mut v2 = v.clone();
    let count = AtomicUsize::new(0);
    let par: Vec<_> = v
        .par_chunk_by_mut(|x, y| {
            count.fetch_add(1, Relaxed);
            (x % 10 < 3) == (y % 10 < 3)
        })
        .collect();
    assert_eq!(count.into_inner(), v2.len() - 1);

    let seq: Vec<_> = v2
        .chunk_by_mut(|x, y| (x % 10 < 3) == (y % 10 < 3))
        .collect();
    assert_eq!(par, seq);
}
