#[test]
fn slice_chunk_by() {
    let v: Vec<_> = (0..1000).collect();
    assert_eq!(v[..0].par_chunk_by(|_, _| todo!()).count(), 0);
    assert_eq!(v[..1].par_chunk_by(|_, _| todo!()).count(), 1);
    assert_eq!(v[..2].par_chunk_by(|_, _| true).count(), 1);
    assert_eq!(v[..2].par_chunk_by(|_, _| false).count(), 2);

    let count = AtomicUsize::new(0);
    let par: Vec<_> = v
        .par_chunk_by(|x, y| {
            count.fetch_add(1, Relaxed);
            (x % 10 < 3) == (y % 10 < 3)
        })
        .collect();
    assert_eq!(count.into_inner(), v.len() - 1);

    let seq: Vec<_> = v.chunk_by(|x, y| (x % 10 < 3) == (y % 10 < 3)).collect();
    assert_eq!(par, seq);
}
