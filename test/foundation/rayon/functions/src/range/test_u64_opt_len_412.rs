#[test]
fn test_u64_opt_len() {
    assert_eq!(Some(100), (0..100u64).into_par_iter().opt_len());
    assert_eq!(
        Some(usize::MAX),
        (0..usize::MAX as u64).into_par_iter().opt_len()
    );
    if (usize::MAX as u64) < u64::MAX {
        assert_eq!(
            None,
            (0..(usize::MAX as u64).wrapping_add(1))
                .into_par_iter()
                .opt_len()
        );
        assert_eq!(None, (0..u64::MAX).into_par_iter().opt_len());
    }
}
