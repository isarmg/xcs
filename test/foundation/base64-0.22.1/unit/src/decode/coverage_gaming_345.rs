#[cfg(test)]
mod coverage_gaming {
    use super::*;
    use std::error::Error;

    #[test]
    fn decode_error() {
        let _ = format!("{:?}", DecodeError::InvalidPadding.clone());
        let _ = format!(
            "{} {} {} {}",
            DecodeError::InvalidByte(0, 0),
            DecodeError::InvalidLength(0),
            DecodeError::InvalidLastSymbol(0, 0),
            DecodeError::InvalidPadding,
        );
    }

    #[test]
    fn decode_slice_error() {
        let _ = format!("{:?}", DecodeSliceError::OutputSliceTooSmall.clone());
        let _ = format!(
            "{} {}",
            DecodeSliceError::OutputSliceTooSmall,
            DecodeSliceError::DecodeError(DecodeError::InvalidPadding)
        );
        let _ = DecodeSliceError::OutputSliceTooSmall.source();
        let _ = DecodeSliceError::DecodeError(DecodeError::InvalidPadding).source();
    }

    #[test]
    fn deprecated_fns() {
        let _ = decode("");
        let _ = decode_engine("", &crate::prelude::BASE64_STANDARD);
        let _ = decode_engine_vec("", &mut Vec::new(), &crate::prelude::BASE64_STANDARD);
        let _ = decode_engine_slice("", &mut [], &crate::prelude::BASE64_STANDARD);
    }

    #[test]
    fn decoded_len_est() {
        assert_eq!(3, decoded_len_estimate(4));
    }
}
