#[test]
#[cfg(target_pointer_width = "64")]
fn test_u32_opt_len() {
    assert_eq!(Some(101), (0..=100u32).into_par_iter().opt_len());
    assert_eq!(
        Some(u32::MAX as usize),
        (0..=u32::MAX - 1).into_par_iter().opt_len()
    );
    assert_eq!(
        Some(u32::MAX as usize + 1),
        (0..=u32::MAX).into_par_iter().opt_len()
    );
}
