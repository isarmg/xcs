#[cfg(test)]
mod tests {
    use super::split_content;
    use std::ffi::OsString;

    // This test ensures that all the parts of the data are split.
    #[test]
    fn test_copy_file() {
        assert_eq!(split_content(b"hello\0"), vec![OsString::from("hello")]);
        assert_eq!(split_content(b"hello"), vec![OsString::from("hello")]);
        assert_eq!(
            split_content(b"hello\0b"),
            vec![OsString::from("hello"), "b".into()]
        );
        assert_eq!(
            split_content(b"hello\0\0\0\0b"),
            vec![OsString::from("hello"), "b".into()]
        );
    }
}
