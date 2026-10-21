#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn non_empty_not_present() {
        assert!(!non_empty(None));
    }

    #[test]
    fn non_empty_empty() {
        assert!(!non_empty(Some(std::ffi::OsStr::new(""))));
    }

    #[test]
    fn non_empty_texty() {
        assert!(non_empty(Some(std::ffi::OsStr::new("hello"))));
    }
}
