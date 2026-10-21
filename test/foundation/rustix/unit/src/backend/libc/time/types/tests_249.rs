#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(any(
        linux_kernel,
        target_os = "freebsd",
        target_os = "fuchsia",
        target_os = "illumos",
        target_os = "netbsd"
    ))]
    #[test]
    fn test_types() {
        static_assertions::assert_eq_size!(TimerfdFlags, c::c_int);
        static_assertions::assert_eq_size!(TimerfdTimerFlags, c::c_int);
    }
}
