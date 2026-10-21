#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(not(any(apple, target_os = "wasi")))]
    #[test]
    fn test_types() {
        static_assertions::assert_eq_size!(PipeFlags, c::c_int);

        #[cfg(linux_kernel)]
        static_assertions::assert_eq_size!(SpliceFlags, c::c_int);
    }
}
