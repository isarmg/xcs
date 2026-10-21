#[test]
fn test_par_rchunks_exact_remainder() {
    let v: &[i32] = &[0, 1, 2, 3, 4];
    let c = v.par_rchunks_exact(2);
    assert_eq!(c.remainder(), &[0]);
    assert_eq!(c.len(), 2);
}
