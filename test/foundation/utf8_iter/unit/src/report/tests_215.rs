#[cfg(test)]
mod tests {
    use crate::ErrorReportingUtf8Chars;

    // Should be a static assert, but not taking a dependency for this.
    #[test]
    fn test_size() {
        assert_eq!(
            core::mem::size_of::<Option<<ErrorReportingUtf8Chars<'_> as Iterator>::Item>>(),
            core::mem::size_of::<Option<char>>()
        );
    }

    #[test]
    fn test_eq() {
        let a: <ErrorReportingUtf8Chars<'_> as Iterator>::Item = Ok('a');
        let a_again: <ErrorReportingUtf8Chars<'_> as Iterator>::Item = Ok('a');
        assert_eq!(a, a_again);
    }
}
