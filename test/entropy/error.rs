mod entropy_error_tests {
    use super::Error;
    use core::num::NonZeroU32;

    #[test]
    fn explicit_user_codes_remain_distinct() {
        let code = NonZeroU32::new(Error::CUSTOM_START + 19).unwrap();
        let error = Error::from(code);
        assert_eq!(error.code(), Some(code));
        assert_eq!(error.raw_os_error(), None);
    }

    #[test]
    fn explicit_os_codes_survive() {
        let error = Error::from(NonZeroU32::new(13).unwrap());
        assert_eq!(error.raw_os_error(), Some(13));
    }

    #[cfg(feature = "system-entropy")]
    #[test]
    fn entropy_errors_retain_identity_and_diagnostics() {
        extern crate std;
        for original in [system_entropy::Error::UNSUPPORTED, system_entropy::Error::UNEXPECTED,
                         system_entropy::Error::ERRNO_NOT_POSITIVE, system_entropy::Error::new_custom(37)] {
            let error = Error::from(original);
            assert_eq!(error.code(), None);
            assert_eq!(error.raw_os_error(), original.raw_os_error());
            assert_eq!(std::format!("{}", error), std::format!("{}", original));
            assert_eq!(std::format!("{:?}", error), std::format!("{:?}", original));
            #[cfg(feature = "std")]
            {
                assert_eq!(error.inner().downcast_ref::<system_entropy::Error>(), Some(&original));
                assert!(std::error::Error::source(&error).is_some());
                let io: std::io::Error = error.into();
                assert_eq!(io.kind(), std::io::ErrorKind::Other);
            }
        }
    }
}
