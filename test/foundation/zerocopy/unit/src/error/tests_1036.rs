#[cfg(test)]
mod tests {
    use core::convert::Infallible;

    use super::*;

    #[test]
    fn test_send_sync() {
        // Test that all error types are `Send + Sync` even if `Dst: !Send +
        // !Sync`.

        #[allow(dead_code)]
        fn is_send_sync<T: Send + Sync>(_t: T) {}

        #[allow(dead_code)]
        fn alignment_err_is_send_sync<Src: Send + Sync, Dst>(err: AlignmentError<Src, Dst>) {
            is_send_sync(err)
        }

        #[allow(dead_code)]
        fn size_err_is_send_sync<Src: Send + Sync, Dst>(err: SizeError<Src, Dst>) {
            is_send_sync(err)
        }

        #[allow(dead_code)]
        fn validity_err_is_send_sync<Src: Send + Sync, Dst: TryFromBytes>(
            err: ValidityError<Src, Dst>,
        ) {
            is_send_sync(err)
        }

        #[allow(dead_code)]
        fn convert_error_is_send_sync<Src: Send + Sync, Dst: TryFromBytes>(
            err: ConvertError<
                AlignmentError<Src, Dst>,
                SizeError<Src, Dst>,
                ValidityError<Src, Dst>,
            >,
        ) {
            is_send_sync(err)
        }
    }

    #[test]
    fn test_eq_partial_eq_clone() {
        // Test that all error types implement `Eq`, `PartialEq`
        // and `Clone` if src does
        // even if `Dst: !Eq`, `!PartialEq`, `!Clone`.

        #[allow(dead_code)]
        fn is_eq_partial_eq_clone<T: Eq + PartialEq + Clone>(_t: T) {}

        #[allow(dead_code)]
        fn alignment_err_is_eq_partial_eq_clone<Src: Eq + PartialEq + Clone, Dst>(
            err: AlignmentError<Src, Dst>,
        ) {
            is_eq_partial_eq_clone(err)
        }

        #[allow(dead_code)]
        fn size_err_is_eq_partial_eq_clone<Src: Eq + PartialEq + Clone, Dst>(
            err: SizeError<Src, Dst>,
        ) {
            is_eq_partial_eq_clone(err)
        }

        #[allow(dead_code)]
        fn validity_err_is_eq_partial_eq_clone<Src: Eq + PartialEq + Clone, Dst: TryFromBytes>(
            err: ValidityError<Src, Dst>,
        ) {
            is_eq_partial_eq_clone(err)
        }

        #[allow(dead_code)]
        fn convert_error_is_eq_partial_eq_clone<Src: Eq + PartialEq + Clone, Dst: TryFromBytes>(
            err: ConvertError<
                AlignmentError<Src, Dst>,
                SizeError<Src, Dst>,
                ValidityError<Src, Dst>,
            >,
        ) {
            is_eq_partial_eq_clone(err)
        }
    }

    #[test]
    fn alignment_display() {
        #[repr(C, align(128))]
        struct Aligned {
            bytes: [u8; 128],
        }

        impl_known_layout!(elain::Align::<8>);

        let aligned = Aligned { bytes: [0; 128] };

        let bytes = &aligned.bytes[1..];
        let addr = crate::util::AsAddress::addr(bytes);
        assert_eq!(
            AlignmentError::<_, elain::Align::<8>>::new_checked(bytes).to_string(),
            format!("The conversion failed because the address of the source is not a multiple of the alignment of the destination type.\n\
            \nSource type: &[u8]\
            \nSource address: 0x{:x} (a multiple of 1)\
            \nDestination type: elain::Align<8>\
            \nDestination alignment: 8", addr)
        );

        let bytes = &aligned.bytes[2..];
        let addr = crate::util::AsAddress::addr(bytes);
        assert_eq!(
            AlignmentError::<_, elain::Align::<8>>::new_checked(bytes).to_string(),
            format!("The conversion failed because the address of the source is not a multiple of the alignment of the destination type.\n\
            \nSource type: &[u8]\
            \nSource address: 0x{:x} (a multiple of 2)\
            \nDestination type: elain::Align<8>\
            \nDestination alignment: 8", addr)
        );

        let bytes = &aligned.bytes[3..];
        let addr = crate::util::AsAddress::addr(bytes);
        assert_eq!(
            AlignmentError::<_, elain::Align::<8>>::new_checked(bytes).to_string(),
            format!("The conversion failed because the address of the source is not a multiple of the alignment of the destination type.\n\
            \nSource type: &[u8]\
            \nSource address: 0x{:x} (a multiple of 1)\
            \nDestination type: elain::Align<8>\
            \nDestination alignment: 8", addr)
        );

        let bytes = &aligned.bytes[4..];
        let addr = crate::util::AsAddress::addr(bytes);
        assert_eq!(
            AlignmentError::<_, elain::Align::<8>>::new_checked(bytes).to_string(),
            format!("The conversion failed because the address of the source is not a multiple of the alignment of the destination type.\n\
            \nSource type: &[u8]\
            \nSource address: 0x{:x} (a multiple of 4)\
            \nDestination type: elain::Align<8>\
            \nDestination alignment: 8", addr)
        );
    }

    #[test]
    fn size_display() {
        assert_eq!(
            SizeError::<_, [u8]>::new(&[0u8; 2][..]).to_string(),
            "The conversion failed because the source was incorrectly sized to complete the conversion into the destination type.\n\
            \nSource type: &[u8]\
            \nSource size: 2 bytes\
            \nDestination type: [u8]"
        );

        assert_eq!(
            SizeError::<_, [u8; 2]>::new(&[0u8; 1][..]).to_string(),
            "The conversion failed because the source was incorrectly sized to complete the conversion into the destination type.\n\
            \nSource type: &[u8]\
            \nSource size: 1 byte\
            \nDestination size: 2 bytes\
            \nDestination type: [u8; 2]"
        );
    }

    #[test]
    fn validity_display() {
        assert_eq!(
            ValidityError::<_, bool>::new(&[2u8; 1][..]).to_string(),
            "The conversion failed because the source bytes are not a valid value of the destination type.\n\
            \n\
            Destination type: bool"
        );
    }

    #[test]
    fn test_convert_error_debug() {
        let err: ConvertError<
            AlignmentError<&[u8], u16>,
            SizeError<&[u8], u16>,
            ValidityError<&[u8], bool>,
        > = ConvertError::Alignment(AlignmentError::new_checked(&[0u8]));
        assert_eq!(format!("{:?}", err), "Alignment(AlignmentError)");

        let err: ConvertError<
            AlignmentError<&[u8], u16>,
            SizeError<&[u8], u16>,
            ValidityError<&[u8], bool>,
        > = ConvertError::Size(SizeError::new(&[0u8]));
        assert_eq!(format!("{:?}", err), "Size(SizeError)");

        let err: ConvertError<
            AlignmentError<&[u8], u16>,
            SizeError<&[u8], u16>,
            ValidityError<&[u8], bool>,
        > = ConvertError::Validity(ValidityError::new(&[0u8]));
        assert_eq!(format!("{:?}", err), "Validity(ValidityError)");
    }

    #[test]
    fn test_convert_error_from_unaligned() {
        // u8 is Unaligned
        let err: ConvertError<
            AlignmentError<&[u8], u8>,
            SizeError<&[u8], u8>,
            ValidityError<&[u8], bool>,
        > = ConvertError::Size(SizeError::new(&[0u8]));
        let converted: ConvertError<Infallible, SizeError<&[u8], u8>, ValidityError<&[u8], bool>> =
            ConvertError::from(err);
        match converted {
            ConvertError::Size(_) => {}
            _ => panic!("Expected Size error"),
        }
    }

    #[test]
    fn test_alignment_error_display_debug() {
        let err: AlignmentError<&[u8], u16> = AlignmentError::new_checked(&[0u8]);
        assert!(format!("{:?}", err).contains("AlignmentError"));
        assert!(format!("{}", err).contains("address of the source is not a multiple"));
    }

    #[test]
    fn test_size_error_display_debug() {
        let err: SizeError<&[u8], u16> = SizeError::new(&[0u8]);
        assert!(format!("{:?}", err).contains("SizeError"));
        assert!(format!("{}", err).contains("source was incorrectly sized"));
    }

    #[test]
    fn test_validity_error_display_debug() {
        let err: ValidityError<&[u8], bool> = ValidityError::new(&[0u8]);
        assert!(format!("{:?}", err).contains("ValidityError"));
        assert!(format!("{}", err).contains("source bytes are not a valid value"));
    }

    #[test]
    fn test_convert_error_display_debug_more() {
        let err: ConvertError<
            AlignmentError<&[u8], u16>,
            SizeError<&[u8], u16>,
            ValidityError<&[u8], bool>,
        > = ConvertError::Alignment(AlignmentError::new_checked(&[0u8]));
        assert!(format!("{}", err).contains("address of the source is not a multiple"));

        let err: ConvertError<
            AlignmentError<&[u8], u16>,
            SizeError<&[u8], u16>,
            ValidityError<&[u8], bool>,
        > = ConvertError::Size(SizeError::new(&[0u8]));
        assert!(format!("{}", err).contains("source was incorrectly sized"));

        let err: ConvertError<
            AlignmentError<&[u8], u16>,
            SizeError<&[u8], u16>,
            ValidityError<&[u8], bool>,
        > = ConvertError::Validity(ValidityError::new(&[0u8]));
        assert!(format!("{}", err).contains("source bytes are not a valid value"));
    }

    #[test]
    fn test_alignment_error_methods() {
        let err: AlignmentError<&[u8], u16> = AlignmentError::new_checked(&[0u8]);

        // into_src
        let src = err.clone().into_src();
        assert_eq!(src, &[0u8]);

        // into
        let converted: ConvertError<
            AlignmentError<&[u8], u16>,
            SizeError<&[u8], u16>,
            ValidityError<&[u8], bool>,
        > = err.clone().into();
        match converted {
            ConvertError::Alignment(_) => {}
            _ => panic!("Expected Alignment error"),
        }

        // clone
        let cloned = err.clone();
        assert_eq!(err, cloned);

        // eq
        assert_eq!(err, cloned);
        let err2: AlignmentError<&[u8], u16> = AlignmentError::new_checked(&[1u8]);
        assert_ne!(err, err2);
    }

    #[test]
    fn test_convert_error_from_unaligned_variants() {
        // u8 is Unaligned
        let err: ConvertError<
            AlignmentError<&[u8], u8>,
            SizeError<&[u8], u8>,
            ValidityError<&[u8], bool>,
        > = ConvertError::Validity(ValidityError::new(&[0u8]));
        let converted: ConvertError<Infallible, SizeError<&[u8], u8>, ValidityError<&[u8], bool>> =
            ConvertError::from(err);
        match converted {
            ConvertError::Validity(_) => {}
            _ => panic!("Expected Validity error"),
        }

        let err: ConvertError<
            AlignmentError<&[u8], u8>,
            SizeError<&[u8], u8>,
            ValidityError<&[u8], bool>,
        > = ConvertError::Size(SizeError::new(&[0u8]));
        let converted: ConvertError<Infallible, SizeError<&[u8], u8>, ValidityError<&[u8], bool>> =
            ConvertError::from(err);
        match converted {
            ConvertError::Size(_) => {}
            _ => panic!("Expected Size error"),
        }
    }
}
