#[cfg(test)]
mod tests {
    use super::*;

    // We test that our 'MatchError' type is the size we expect. This isn't an
    // API guarantee, but if the size increases, we really want to make sure we
    // decide to do that intentionally. So this should be a speed bump. And in
    // general, we should not increase the size without a very good reason.
    //
    // Why? Because low level search APIs return Result<.., MatchError>. When
    // MatchError gets bigger, so to does the Result type.
    //
    // Now, when 'alloc' is enabled, we do box the error, which de-emphasizes
    // the importance of keeping a small error type. But without 'alloc', we
    // still want things to be small.
    #[test]
    fn match_error_size() {
        let expected_size = if cfg!(feature = "alloc") {
            core::mem::size_of::<usize>()
        } else {
            2 * core::mem::size_of::<usize>()
        };
        assert_eq!(expected_size, core::mem::size_of::<MatchError>());
    }

    // Same as above, but for the underlying match error kind.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn match_error_kind_size() {
        let expected_size = 2 * core::mem::size_of::<usize>();
        assert_eq!(expected_size, core::mem::size_of::<MatchErrorKind>());
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn match_error_kind_size() {
        let expected_size = 3 * core::mem::size_of::<usize>();
        assert_eq!(expected_size, core::mem::size_of::<MatchErrorKind>());
    }

    #[test]
    fn incorrect_asref_guard() {
        struct Bad(std::cell::Cell<bool>);

        impl AsRef<[u8]> for Bad {
            fn as_ref(&self) -> &[u8] {
                if self.0.replace(false) {
                    &[]
                } else {
                    &[0; 1000]
                }
            }
        }

        let bad = Bad(std::cell::Cell::new(true));
        let input = Input::new(&bad);
        assert!(input.end() <= input.haystack().len());
    }
}
