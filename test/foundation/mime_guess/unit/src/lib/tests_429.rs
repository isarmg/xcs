#[cfg(test)]
mod tests {
    include!("mime_types.rs");

    use super::{expect_mime, from_ext, from_path, get_mime_extensions_str};
    #[allow(deprecated, unused_imports)]
    use std::ascii::AsciiExt;

    use std::fmt::Debug;
    use std::path::Path;

    #[test]
    fn check_type_bounds() {
        fn assert_type_bounds<T: Clone + Debug + Send + Sync + 'static>() {}

        assert_type_bounds::<super::MimeGuess>();
        assert_type_bounds::<super::Iter>();
        assert_type_bounds::<super::IterRaw>();
    }

    #[test]
    fn test_mime_type_guessing() {
        assert_eq!(
            from_ext("gif").first_or_octet_stream().to_string(),
            "image/gif".to_string()
        );
        assert_eq!(
            from_ext("TXT").first_or_octet_stream().to_string(),
            "text/plain".to_string()
        );
        assert_eq!(
            from_ext("blahblah").first_or_octet_stream().to_string(),
            "application/octet-stream".to_string()
        );

        assert_eq!(
            from_path(Path::new("/path/to/file.gif"))
                .first_or_octet_stream()
                .to_string(),
            "image/gif".to_string()
        );
        assert_eq!(
            from_path("/path/to/file.gif")
                .first_or_octet_stream()
                .to_string(),
            "image/gif".to_string()
        );
    }

    #[test]
    fn test_mime_type_guessing_opt() {
        assert_eq!(
            from_ext("gif").first().unwrap().to_string(),
            "image/gif".to_string()
        );
        assert_eq!(
            from_ext("TXT").first().unwrap().to_string(),
            "text/plain".to_string()
        );
        assert_eq!(from_ext("blahblah").first(), None);

        assert_eq!(
            from_path("/path/to/file.gif").first().unwrap().to_string(),
            "image/gif".to_string()
        );
        assert_eq!(from_path("/path/to/file").first(), None);
    }

    #[test]
    fn test_are_mime_types_parseable() {
        for (_, mimes) in MIME_TYPES {
            mimes.iter().for_each(|s| {
                expect_mime(s);
            });
        }
    }

    // RFC: Is this test necessary anymore? --@cybergeek94, 2/1/2016
    #[test]
    fn test_are_extensions_ascii() {
        for (ext, _) in MIME_TYPES {
            assert!(ext.is_ascii(), "Extension not ASCII: {:?}", ext);
        }
    }

    #[test]
    fn test_are_extensions_sorted() {
        // simultaneously checks the requirement that duplicate extension entries are adjacent
        for (&(ext, _), &(n_ext, _)) in MIME_TYPES.iter().zip(MIME_TYPES.iter().skip(1)) {
            assert!(
                ext <= n_ext,
                "Extensions in src/mime_types should be sorted lexicographically
                in ascending order. Failed assert: {:?} <= {:?}",
                ext,
                n_ext
            );
        }
    }

    #[test]
    fn test_get_mime_extensions_str_no_panic_if_bad_mime() {
        assert_eq!(get_mime_extensions_str(""), None);
    }
}
