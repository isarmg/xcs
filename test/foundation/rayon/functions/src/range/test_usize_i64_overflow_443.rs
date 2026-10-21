#[test]
#[cfg(target_pointer_width = "64")]
fn test_usize_i64_overflow() {
    use crate::ThreadPoolBuilder;

    let iter = (-2..i64::MAX).into_par_iter();
    assert_eq!(iter.opt_len(), Some(i64::MAX as usize + 2));

    // always run with multiple threads to split into, or this will take forever...
    let pool = ThreadPoolBuilder::new().num_threads(8).build().unwrap();
    pool.install(|| assert_eq!(iter.find_last(|_| true), Some(i64::MAX - 1)));
}
