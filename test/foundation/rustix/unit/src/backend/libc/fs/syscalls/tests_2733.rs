#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sizes() {
        #[cfg(linux_kernel)]
        static_assertions::assert_eq_size!(c::loff_t, u64);

        // Assert that `Timestamps` has the expected layout. If we're not fixing
        // y2038, libc's type should match ours. If we are, it's smaller.
        #[cfg(not(fix_y2038))]
        static_assertions::assert_eq_size!([c::timespec; 2], Timestamps);
        #[cfg(fix_y2038)]
        assert!(core::mem::size_of::<[c::timespec; 2]>() < core::mem::size_of::<Timestamps>());
    }
}
