#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(linux_kernel)]
    fn test_flags() {
        // libc may publicly define `O_LARGEFILE` to 0, but we want the real
        // non-zero value.
        assert_ne!(O_LARGEFILE, 0);
    }
}
