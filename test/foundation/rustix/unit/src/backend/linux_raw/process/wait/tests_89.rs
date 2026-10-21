#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_libc_correspondence() {
        for status in [
            0,
            1,
            63,
            64,
            65,
            127,
            128,
            129,
            255,
            256,
            257,
            4095,
            4096,
            4097,
            i32::MAX,
            i32::MIN,
            u32::MAX as i32,
        ] {
            assert_eq!(WIFSTOPPED(status), libc::WIFSTOPPED(status));
            assert_eq!(WSTOPSIG(status), libc::WSTOPSIG(status));
            assert_eq!(WIFCONTINUED(status), libc::WIFCONTINUED(status));
            assert_eq!(WIFSIGNALED(status), libc::WIFSIGNALED(status));
            assert_eq!(WTERMSIG(status), libc::WTERMSIG(status));
            assert_eq!(WIFEXITED(status), libc::WIFEXITED(status));
            assert_eq!(WEXITSTATUS(status), libc::WEXITSTATUS(status));
        }
    }
}
