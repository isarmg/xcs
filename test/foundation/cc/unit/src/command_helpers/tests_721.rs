#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stderr_warnings_skips_ignored_line() {
        // cl.exe echoes the source file name before any real diagnostics (#896).
        let stderr = b"expando.c\r\nexpando.c(2): warning C4005: 'X': macro redefinition\r\n";
        let warnings: Vec<_> = stderr_warnings(stderr, Some(b"expando.c")).collect();
        assert_eq!(
            warnings,
            [&b"expando.c(2): warning C4005: 'X': macro redefinition"[..]]
        );
    }

    #[test]
    fn stderr_warnings_keeps_every_line_without_ignored_line() {
        let stderr = b"expando.c\n\nsecond\n";
        let warnings: Vec<_> = stderr_warnings(stderr, None).collect();
        assert_eq!(warnings, [&b"expando.c"[..], &b"second"[..]]);
    }
}
