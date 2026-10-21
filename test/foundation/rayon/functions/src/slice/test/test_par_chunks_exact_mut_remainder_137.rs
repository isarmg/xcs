#[test]
fn test_par_chunks_exact_mut_remainder() {
    let v: &mut [i32] = &mut [0, 1, 2, 3, 4];
    let mut c = v.par_chunks_exact_mut(2);
    assert_eq!(c.remainder(), &[4]);
    assert_eq!(c.len(), 2);
    assert_eq!(c.into_remainder(), &[4]);

    let mut c = v.par_chunks_exact_mut(2);
    assert_eq!(c.take_remainder(), &[4]);
    assert_eq!(c.take_remainder(), &[]);
    assert_eq!(c.len(), 2);
}
