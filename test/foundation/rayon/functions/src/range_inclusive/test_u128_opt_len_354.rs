#[test]
fn test_u128_opt_len() {
    assert_eq!(Some(101), (0..=100u128).into_par_iter().opt_len());
    assert_eq!(
        Some(usize::MAX),
        (0..=usize::MAX as u128 - 1).into_par_iter().opt_len()
    );
    assert_eq!(None, (0..=usize::MAX as u128).into_par_iter().opt_len());
    assert_eq!(None, (0..=u128::MAX).into_par_iter().opt_len());
}
