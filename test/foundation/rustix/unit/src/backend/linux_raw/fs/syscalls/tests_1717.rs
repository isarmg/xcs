#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sizes() {
        static_assertions::assert_eq_size!(linux_raw_sys::general::__kernel_loff_t, u64);
        static_assertions::assert_eq_align!(linux_raw_sys::general::__kernel_loff_t, u64);

        // Assert that `Timestamps` has the expected layout.
        static_assertions::assert_eq_size!(
            [linux_raw_sys::general::__kernel_timespec; 2],
            Timestamps
        );
        static_assertions::assert_eq_align!(
            [linux_raw_sys::general::__kernel_timespec; 2],
            Timestamps
        );
    }
}
